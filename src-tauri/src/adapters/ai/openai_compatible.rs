//! Suporte a endpoints compatíveis com a API de chat da OpenAI.
//!
//! Usado por `OpenAiChatBotProvider`, `OpenRouterChatBotProvider` e
//! `CustomChatBotProvider`. Nunca registra a chave de API em logs/erros.

use std::io::BufRead;
use std::time::Duration;

use serde_json::{json, Value};

use crate::domain::llm::{Completion, CompletionRequest, CompletionUsage, ProviderConfig};
use crate::error::{GhostWhisperError, Result};

/// Tempo limite padrão das chamadas (segundos).
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Quantas vezes reenviamos após o provider rejeitar um parâmetro.
const MAX_PARAM_RETRIES: usize = 3;

/// Quantas vezes reenviamos quando o provider responde **sem conteúdo**.
///
/// Respostas vazias costumam ser transitórias (proxy/instabilidade) ou acontecem
/// quando o raciocínio do modelo consumiu todo o orçamento de tokens. Um reenvio
/// único, com o orçamento dobrado, resolve a maioria sem esconder um bloqueio real.
const MAX_EMPTY_RETRIES: usize = 1;
/// Pausa antes do reenvio por resposta vazia (ms).
const EMPTY_RETRY_BACKOFF_MS: u64 = 300;
/// Teto do orçamento de tokens ao crescer para o reenvio.
const MAX_GROWN_TOKENS: u64 = 16_000;

/// Parâmetros **opcionais** que podemos remitir/reenviar sem quebrar a chamada
/// (contrato de amostragem; `model`/`messages` são obrigatórios e nunca removidos).
const REMOVABLE_PARAMS: [&str; 7] = [
    "temperature",
    "top_p",
    "seed",
    "frequency_penalty",
    "presence_penalty",
    "max_tokens",
    "max_completion_tokens",
];

/// Executa `POST {base_url}/chat/completions`.
///
/// Modelos novos rejeitam `max_tokens` (pedem `max_completion_tokens`) e **valores**
/// de `temperature` (modelos de raciocínio só aceitam o padrão). Ao receber um
/// `400` de parâmetro suportado, o campo é renomeado/removido e a chamada
/// reenviada — mesmo tratamento do Perssua (`providerParameterErrors.js`), que
/// classifica `unsupported_parameter` **e** `unsupported_value`.
pub fn chat_completion(
    config: &ProviderConfig,
    base_url: &str,
    request: &CompletionRequest,
    extra_headers: &[(&str, &str)],
) -> Result<Completion> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let mut body = build_body(config, request);
    let mut retries = 0usize;
    let mut empty_retries = 0usize;

    loop {
        let mut call = ureq::post(&url)
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", config.api_key));
        for (name, value) in extra_headers {
            call = call.set(name, value);
        }

        match call.send_json(body.clone()) {
            Ok(response) => {
                let value: Value = response.into_json().map_err(|e| {
                    GhostWhisperError::Other(format!("resposta inválida do provider: {e}"))
                })?;
                let parsed = parse_completion(&value);
                if parsed.text.is_empty() {
                    // Reenvia uma vez com o orçamento de tokens dobrado: cobre
                    // resposta transitória e modelo de raciocínio que gastou tudo
                    // antes de escrever (finish_reason=length).
                    if empty_retries < MAX_EMPTY_RETRIES {
                        empty_retries += 1;
                        let grew = grow_token_budget(&mut body);
                        tracing::debug!(
                            finish_reason = ?parsed.finish_reason,
                            grew_token_budget = grew,
                            "provider respondeu sem texto; reenviando"
                        );
                        std::thread::sleep(Duration::from_millis(EMPTY_RETRY_BACKOFF_MS));
                        continue;
                    }
                    return Err(GhostWhisperError::Other(empty_response_message(&parsed)));
                }
                return Ok(Completion {
                    text: parsed.text,
                    model: config.model.clone(),
                    provider: Some(config.kind.as_str().to_string()),
                    usage: parse_usage(&value),
                });
            }
            Err(ureq::Error::Status(code, response))
                if code == 400 && retries < MAX_PARAM_RETRIES =>
            {
                let detail = response.into_string().unwrap_or_default();
                let Some(param) = rejected_param(&detail) else {
                    return Err(provider_status(code, &detail));
                };
                if !apply_rejected_param_fallback(&mut body, &param) {
                    return Err(provider_status(code, &detail));
                }
                retries += 1;
                tracing::debug!("provider rejeitou '{param}'; reenviando sem ele");
            }
            Err(other) => return Err(map_transport_error(other)),
        }
    }
}

/// Resultado de uma tentativa de streaming.
enum StreamOutcome {
    /// O provider transmitiu o texto; a completude está pronta.
    Done(Completion),
    /// O provider não suporta/ignorou o streaming — cair para a chamada normal.
    Fallback,
}

/// Executa `POST {base_url}/chat/completions` com `stream: true` (SSE),
/// reportando o **texto acumulado** a cada pedaço.
///
/// Cai para [`chat_completion`] (sem streaming) quando o provider não devolve
/// `text/event-stream` ou quando não há conteúdo — assim a sugestão funciona em
/// qualquer endpoint OpenAI-compatível (BR-028).
pub fn chat_completion_streaming(
    config: &ProviderConfig,
    base_url: &str,
    request: &CompletionRequest,
    extra_headers: &[(&str, &str)],
    on_text: &mut dyn FnMut(&str),
) -> Result<Completion> {
    match try_stream(config, base_url, request, extra_headers, on_text)? {
        StreamOutcome::Done(completion) => Ok(completion),
        StreamOutcome::Fallback => {
            let completion = chat_completion(config, base_url, request, extra_headers)?;
            on_text(&completion.text);
            Ok(completion)
        }
    }
}

/// Tenta o streaming uma vez; erros de status viram `Fallback` (o retry por
/// parâmetro rejeitado vive na chamada normal).
fn try_stream(
    config: &ProviderConfig,
    base_url: &str,
    request: &CompletionRequest,
    extra_headers: &[(&str, &str)],
    on_text: &mut dyn FnMut(&str),
) -> Result<StreamOutcome> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let mut body = build_body(config, request);
    body["stream"] = json!(true);

    let mut call = ureq::post(&url)
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .set("Content-Type", "application/json")
        .set("Accept", "text/event-stream")
        .set("Authorization", &format!("Bearer {}", config.api_key));
    for (name, value) in extra_headers {
        call = call.set(name, value);
    }

    let response = match call.send_json(body) {
        Ok(response) => response,
        // 4xx/5xx: deixa a chamada normal tratar (inclusive o retry de parâmetro).
        Err(ureq::Error::Status(_, _)) => return Ok(StreamOutcome::Fallback),
        Err(other) => return Err(map_transport_error(other)),
    };
    let content_type = response
        .header("Content-Type")
        .unwrap_or_default()
        .to_string();
    if !content_type.contains("text/event-stream") {
        return Ok(StreamOutcome::Fallback);
    }

    let reader = response.into_reader();
    let mut accumulated = String::new();
    let mut emitted = String::new();
    let mut saw_data = false;
    // O `usage` chega no **último** pedaço do SSE (o provider o envia junto
    // com o `[DONE]`); guardamos o último que trouxe uso.
    let mut stream_usage: Option<CompletionUsage> = None;
    for line in std::io::BufReader::new(reader).lines() {
        let Ok(line) = line else { break };
        let Some(payload) = line.strip_prefix("data:") else {
            continue;
        };
        let payload = payload.trim();
        if payload.is_empty() {
            continue;
        }
        if payload == "[DONE]" {
            break;
        }
        saw_data = true;
        let Ok(value) = serde_json::from_str::<Value>(payload) else {
            continue;
        };
        if let Some(usage) = parse_usage(&value) {
            stream_usage = Some(usage);
        }
        if let Some(delta) = value["choices"]
            .get(0)
            .and_then(|choice| choice["delta"]["content"].as_str())
        {
            accumulated.push_str(delta);
            let current = accumulated.trim_start();
            if current != emitted {
                on_text(current);
                emitted = current.to_string();
            }
        }
    }

    if !saw_data || accumulated.trim().is_empty() {
        return Ok(StreamOutcome::Fallback);
    }
    Ok(StreamOutcome::Done(Completion {
        text: accumulated.trim().to_string(),
        model: config.model.clone(),
        provider: Some(config.kind.as_str().to_string()),
        usage: stream_usage,
    }))
}

/// Lê o bloco `usage` da resposta (tokens + raciocínio, quando houver).
///
/// O formato é o da API da OpenAI e compatível: `usage.prompt_tokens`,
/// `usage.completion_tokens`, `usage.total_tokens` e
/// `usage.completion_tokens_details.reasoning_tokens` (modelos de raciocínio).
fn parse_usage(value: &Value) -> Option<CompletionUsage> {
    let usage = value.get("usage")?;
    let prompt_tokens = usage
        .get("prompt_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let completion_tokens = usage
        .get("completion_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let reasoning_tokens = usage
        .get("completion_tokens_details")
        .and_then(|details| details.get("reasoning_tokens"))
        .and_then(Value::as_u64)
        .map(|value| value as u32);
    if prompt_tokens == 0 && completion_tokens == 0 && total_tokens == 0 {
        return None;
    }
    Some(CompletionUsage {
        prompt_tokens,
        completion_tokens,
        total_tokens,
        reasoning_tokens,
    })
}

/// Monta o corpo da requisição a partir do pedido.
fn build_body(config: &ProviderConfig, request: &CompletionRequest) -> Value {
    let mut messages: Vec<Value> = Vec::new();
    if let Some(system) = &request.system {
        messages.push(json!({ "role": "system", "content": system }));
    }
    messages.push(json!({ "role": "user", "content": user_content(request) }));

    let mut body = json!({ "model": config.model, "messages": messages });
    if let Some(max_tokens) = request.max_tokens {
        body["max_tokens"] = json!(max_tokens);
    }
    // Perssua (`builtInOmissions`): modelos de racio cínio rejeitam `temperature`
    // distinta do padrão — não enviamos o parâmetro para esses modelos.
    if let Some(temperature) = request.temperature {
        if temperature_supported(&config.model) {
            body["temperature"] = json!(temperature);
        }
    }
    body
}

/// Conteúdo da mensagem do usuário.
///
/// Sem imagem, é o **texto puro** (o corpo de sempre — nenhum provider
/// existente é afetado). Com imagem, vira a lista de partes do formato
/// multimodal (`input_text` + `input_image` com data URL), o mesmo formato do
/// Perssua (`userContentParts`).
fn user_content(request: &CompletionRequest) -> Value {
    if request.images.is_empty() {
        return json!(request.prompt);
    }
    let mut parts: Vec<Value> = vec![json!({ "type": "text", "text": request.prompt })];
    for image in &request.images {
        parts.push(json!({
            "type": "image_url",
            "image_url": { "url": data_url(&image.mime, &image.data) }
        }));
    }
    json!(parts)
}

/// Data URL de uma imagem (`data:<mime>;base64,<dados>`).
fn data_url(mime: &str, data: &[u8]) -> String {
    use base64::Engine;
    format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(data)
    )
}

/// Extrai o texto da primeira escolha da resposta.
/// Conteúdo extraído de uma resposta de chat, com o motivo de eventual vazio.
struct ParsedCompletion {
    text: String,
    finish_reason: Option<String>,
    refusal: Option<String>,
}

/// Extrai o texto e o diagnóstico de uma resposta de chat.
fn parse_completion(value: &Value) -> ParsedCompletion {
    let choice = &value["choices"][0];
    ParsedCompletion {
        text: extract_text(value),
        finish_reason: choice["finish_reason"].as_str().map(str::to_string),
        refusal: choice["message"]["refusal"].as_str().map(str::to_string),
    }
}

/// Texto de uma resposta de chat, aceitando os formatos que os endpoints usam.
///
/// `message.content` pode ser string (caso comum), **array de partes**
/// (`[{"type":"text","text":"…"}]`, usado por endpoints multimodais/proxies) ou
/// `null`; a resposta pode ainda vir no formato legado `choices[0].text`.
fn extract_text(value: &Value) -> String {
    let choice = &value["choices"][0];

    if let Some(text) = choice["message"]["content"].as_str() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(parts) = choice["message"]["content"].as_array() {
        let joined: String = parts
            .iter()
            .filter_map(|part| part["text"].as_str().or_else(|| part["content"].as_str()))
            .collect();
        let trimmed = joined.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(text) = choice["text"].as_str() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    String::new()
}

/// Dobra o orçamento de tokens do corpo (para o reenvio por resposta vazia).
///
/// Funciona com `max_tokens` e `max_completion_tokens`; sem orçamento explícito
/// não mexe (não inventa limite que o provider não pediu). Devolve `true` quando
/// o valor realmente cresceu.
fn grow_token_budget(body: &mut Value) -> bool {
    for key in ["max_completion_tokens", "max_tokens"] {
        if let Some(current) = body.get(key).and_then(Value::as_u64) {
            let grown = current.saturating_mul(2).min(MAX_GROWN_TOKENS);
            if grown > current {
                body[key] = json!(grown);
                return true;
            }
        }
    }
    false
}

/// Mensagem de erro acionável para uma resposta vazia.
fn empty_response_message(parsed: &ParsedCompletion) -> String {
    if let Some(refusal) = &parsed.refusal {
        return format!("provider recusou a resposta: {refusal}");
    }
    match parsed.finish_reason.as_deref() {
        Some("length") => "provider esgotou o limite de tokens antes de escrever \
            (finish_reason=length) — aumente o limite de tokens ou use um modelo sem raciocínio"
            .to_string(),
        Some("content_filter") => {
            "provider bloqueou a resposta pelo filtro de conteúdo (finish_reason=content_filter)"
                .to_string()
        }
        Some(reason) if !reason.is_empty() => {
            format!("provider retornou uma resposta vazia (finish_reason={reason})")
        }
        _ => "provider retornou uma resposta vazia".to_string(),
    }
}

/// Descobre qual parâmetro o provider recusou, se a falha for de parâmetro/valor.
///
/// Transposto de `providerParameterErrors.js` (`rejectionCategory`): OpenAI
/// devolve `unsupported_parameter` para parâmetros desconhecidos e
/// `unsupported_value` para **valores** rejeitados (ex.: `temperature` não
/// padrão em modelos gpt-5/o1/o3). Só classificamos erros de 400; quem decide
/// remitir é [`rejected_param`] + [`apply_rejected_param_fallback`], que exigem
/// o parâmetro presente no corpo e na lista de opcionais.
fn rejected_param(detail: &str) -> Option<String> {
    let value: Value = serde_json::from_str(detail).ok()?;
    let error = &value["error"];
    let code = error["code"].as_str().unwrap_or_default();
    let message = error["message"].as_str().unwrap_or_default();
    let param = error["param"].as_str().unwrap_or_default();

    let unsupported_parameter = code == "unsupported_parameter"
        || code == "unknown_parameter"
        || code == "unrecognized_parameter"
        || message.contains("Unsupported parameter")
        || message.contains("unknown parameter")
        || message.contains("unrecognized parameter");
    let unsupported_value = code == "unsupported_value"
        || code == "invalid_value"
        || message.contains("Unsupported value")
        || message.contains("Only the default")
        || message.contains("does not support");
    if !unsupported_parameter && !unsupported_value {
        return None;
    }

    // 1) Campo estruturado (`param`), como o OpenAI devolve.
    if !param.is_empty() {
        return Some(param.to_string());
    }
    // 2) Mensagem em prosa: "Unsupported (parameter|value): 'x' …".
    for marker in ["Unsupported parameter: '", "Unsupported value: '"] {
        if let Some(start) = message.find(marker).map(|at| at + marker.len()) {
            if let Some(end) = message[start..].find('\'') {
                return Some(message[start..start + end].to_string());
            }
        }
    }
    None
}

/// Ajusta o corpo após uma rejeição: renomeia `max_tokens` e remitir os demais.
/// Só mexe em parâmetros **opcionais de amostragem/limite** que estavam no
/// corpo; se não houver nada a remitir, a chamada não é reenviada.
fn apply_rejected_param_fallback(body: &mut Value, param: &str) -> bool {
    if !REMOVABLE_PARAMS.contains(&param) {
        return false;
    }
    let Some(object) = body.as_object_mut() else {
        return false;
    };
    if param == "max_tokens" {
        if let Some(value) = object.remove("max_tokens") {
            object.insert("max_completion_tokens".to_string(), value);
            return true;
        }
        return object.contains_key("max_completion_tokens");
    }
    object.remove(param).is_some()
}

/// Converte erros de transporte em `GhostWhisperError` sem vazar segredos.
pub(crate) fn map_transport_error(error: ureq::Error) -> GhostWhisperError {
    match error {
        ureq::Error::Status(code, response) => {
            let detail = response
                .into_string()
                .map(|body| truncate(&body, 300))
                .unwrap_or_default();
            GhostWhisperError::ProviderStatus { code, detail }
        }
        other => GhostWhisperError::Other(format!("falha de rede ao chamar o provider: {other}")),
    }
}

/// Erro de status do provider com o **código tipado** (BR-045).
///
/// O detalhe é truncado; a chave de API nunca aparece (o header de
/// autorização não volta na resposta).
pub(crate) fn provider_status(code: u16, detail: &str) -> GhostWhisperError {
    GhostWhisperError::ProviderStatus {
        code,
        detail: truncate(detail, 300),
    }
}

pub(crate) fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push('…');
    out
}

/// Indica se o modelo aceita `temperature` diferente do padrão.
///
/// Modelos de raciocínio (perssua `builtInOmissions` para `codex`/gpt-5) só
/// suportam o valor padrão (`1`); para eles o parâmetro nem é enviado.
/// O `deepseek-reasoner` é da mesma família (BR-046).
fn temperature_supported(model: &str) -> bool {
    let name = model.trim().to_ascii_lowercase();
    !(name.starts_with("gpt-5")
        || name.starts_with("o1")
        || name.starts_with("o3")
        || name.starts_with("o4")
        || name.starts_with("deepseek-reasoner"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_TOKENS_400: &str = r#"{"error":{"message":"Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead.","type":"invalid_request_error","param":"max_tokens","code":"unsupported_parameter"}}"#;

    /// Erro real devolvido pelo OpenAI para modelos gpt-5/o* (issue do usuário).
    const TEMPERATURE_VALUE_400: &str = r#"{"error":{"message":"Unsupported value: 'temperature' does not support 0.30000000000000004 with this model. Only the default (1) value is supported.","type":"invalid_request_error","param":"temperature","code":"unsupported_value"}}"#;

    fn body_with_max_tokens() -> Value {
        json!({ "model": "m", "messages": [], "max_tokens": 16, "temperature": 0.0 })
    }

    #[test]
    fn streams_sse_deltas_as_accumulated_text() {
        use crate::domain::settings::ProviderKind;
        use std::io::Write;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut discard = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut discard);
            let body = concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\"> olá\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\", tudo bem?\"}}]}\n\n",
                "data: [DONE]\n\n"
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{body}"
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });

        let config = ProviderConfig {
            kind: ProviderKind::OpenAi,
            model: "m".into(),
            base_url: None,
            api_key: "k".into(),
        };
        let mut seen: Vec<String> = Vec::new();
        let completion = chat_completion_streaming(
            &config,
            &format!("http://{addr}"),
            &CompletionRequest::user("oi"),
            &[],
            &mut |text| seen.push(text.to_string()),
        )
        .expect("stream");

        assert_eq!(completion.text, "> olá, tudo bem?");
        assert_eq!(seen, vec!["> olá", "> olá, tudo bem?"]);
    }

    #[test]
    fn falls_back_when_the_provider_is_not_sse() {
        use crate::domain::settings::ProviderKind;
        use std::io::Write;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            // Duas conexões: a tentativa de streaming e a chamada normal.
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut discard = [0u8; 4096];
                let _ = std::io::Read::read(&mut stream, &mut discard);
                let body = r#"{"choices":[{"message":{"content":"resposta única"}}]}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });

        let config = ProviderConfig {
            kind: ProviderKind::OpenAi,
            model: "m".into(),
            base_url: None,
            api_key: "k".into(),
        };
        let mut seen: Vec<String> = Vec::new();
        let completion = chat_completion_streaming(
            &config,
            &format!("http://{addr}"),
            &CompletionRequest::user("oi"),
            &[],
            &mut |text| seen.push(text.to_string()),
        )
        .expect("fallback");

        assert_eq!(completion.text, "resposta única");
        assert_eq!(seen, vec!["resposta única"]);
    }

    #[test]
    fn reads_the_usage_with_reasoning_tokens() {
        let value = json!({
            "choices": [{"message": {"content": "ok"}}],
            "usage": {
                "prompt_tokens": 120,
                "completion_tokens": 40,
                "total_tokens": 160,
                "completion_tokens_details": { "reasoning_tokens": 24 }
            }
        });
        let usage = parse_usage(&value).expect("usage");
        assert_eq!(usage.prompt_tokens, 120);
        assert_eq!(usage.completion_tokens, 40);
        assert_eq!(usage.total_tokens, 160);
        assert_eq!(usage.reasoning_tokens, Some(24));
        assert!(usage.has_reasoning());
    }

    #[test]
    fn reads_the_usage_without_reasoning() {
        let value = json!({
            "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 }
        });
        let usage = parse_usage(&value).expect("usage");
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.reasoning_tokens, None);
        assert!(!usage.has_reasoning());
    }

    #[test]
    fn no_usage_block_means_no_usage() {
        assert!(parse_usage(&json!({ "choices": [] })).is_none());
        assert!(parse_usage(&json!({ "usage": {} })).is_none());
    }

    #[test]
    fn detects_unsupported_param_from_code_and_field() {
        assert_eq!(
            rejected_param(MAX_TOKENS_400).as_deref(),
            Some("max_tokens")
        );
    }

    #[test]
    fn detects_unsupported_param_from_message_only() {
        let detail =
            r#"{"error":{"message":"Unsupported parameter: 'temperature' is not supported."}}"#;
        assert_eq!(rejected_param(detail).as_deref(), Some("temperature"));
    }

    #[test]
    fn detects_rejected_temperature_value() {
        // Cenário do usuário: `unsupported_value` para models de raciocínio.
        assert_eq!(
            rejected_param(TEMPERATURE_VALUE_400).as_deref(),
            Some("temperature")
        );
    }

    #[test]
    fn detects_unsupported_value_from_message_only() {
        let detail = r#"{"error":{"message":"Unsupported value: 'temperature' does not support 0.3 with this model."}}"#;
        assert_eq!(rejected_param(detail).as_deref(), Some("temperature"));
    }

    #[test]
    fn ignores_unrelated_errors() {
        let detail = r#"{"error":{"message":"The model 'x' does not exist","param":"model","code":"model_not_found"}}"#;
        assert_eq!(rejected_param(detail), None);
    }

    #[test]
    fn ignores_value_error_for_required_param() {
        // `model` é obrigatório: não há retry possível.
        let detail = r#"{"error":{"message":"Unsupported value: 'model' does not support 'x'.","param":"model","code":"unsupported_value"}}"#;
        assert!(!apply_rejected_param_fallback(
            &mut body_with_max_tokens(),
            "model"
        ));
        assert!(rejected_param(detail).is_some());
    }

    #[test]
    fn removes_unsupported_value_params() {
        let mut body = body_with_max_tokens();
        assert!(apply_rejected_param_fallback(&mut body, "temperature"));
        assert!(body.get("temperature").is_none());
        assert_eq!(body["max_tokens"], json!(16));

        // Parâmetro ausente → nada a ajustar (chamada não reenviada).
        let mut other = json!({ "model": "m", "messages": [] });
        assert!(!apply_rejected_param_fallback(&mut other, "temperature"));
    }

    #[test]
    fn renames_max_tokens_to_max_completion_tokens() {
        let mut body = body_with_max_tokens();
        assert!(apply_rejected_param_fallback(&mut body, "max_tokens"));
        assert!(body.get("max_tokens").is_none());
        assert_eq!(body["max_completion_tokens"], json!(16));
        // Idempotente: repetir não muda nada nem falha.
        assert!(apply_rejected_param_fallback(&mut body, "max_tokens"));
        assert_eq!(body["max_completion_tokens"], json!(16));
    }

    #[test]
    fn extracts_first_choice_text() {
        let value = json!({"choices":[{"message":{"content":"  ok  "}}]});
        assert_eq!(extract_text(&value), "ok");
        assert_eq!(extract_text(&json!({})), "");
    }

    #[test]
    fn omits_temperature_for_reasoning_models() {
        for model in ["gpt-5.1", "GPT-5", "o3-mini", "o1-preview"] {
            let config = ProviderConfig {
                kind: crate::domain::settings::ProviderKind::OpenAi,
                model: model.into(),
                base_url: None,
                api_key: "k".into(),
            };
            let request = CompletionRequest::user("oi").with_temperature(0.3);
            let body = build_body(&config, &request);
            assert!(
                body.get("temperature").is_none(),
                "{model} leva temperature"
            );
        }

        let config = ProviderConfig {
            kind: crate::domain::settings::ProviderKind::OpenAi,
            model: "gpt-4o-mini".into(),
            base_url: None,
            api_key: "k".into(),
        };
        let request = CompletionRequest::user("oi").with_temperature(0.3);
        let body = build_body(&config, &request);
        let sent = body["temperature"].as_f64().expect("temperature");
        assert!(
            (sent - 0.3).abs() < 1e-6,
            "gpt-4o-mini deve manter temperature: {sent}"
        );
    }

    #[test]
    fn retries_after_unsupported_max_tokens() {
        use crate::adapters::ai::custom_chat_bot_provider::CustomChatBotProvider;
        use crate::application::test_support::spawn_openai_sequence_mock;
        use crate::domain::settings::ProviderKind;
        use crate::ports::chat_bot_provider::ChatBotProvider;

        let (base_url, seen) = spawn_openai_sequence_mock(vec![
            (400, MAX_TOKENS_400.to_string()),
            (
                200,
                r#"{"choices":[{"message":{"content":"ok"}}]}"#.to_string(),
            ),
        ]);
        let provider = CustomChatBotProvider::new(ProviderConfig {
            kind: ProviderKind::Custom,
            model: "gpt-6-luna".into(),
            base_url: Some(base_url),
            api_key: "test-key".into(),
        });

        let request = CompletionRequest::user("oi")
            .with_max_tokens(16)
            .with_temperature(0.2);
        assert_eq!(provider.complete(&request).expect("complete").text, "ok");

        let requests = seen.lock().expect("lock").clone();
        assert_eq!(requests.len(), 2, "deve reenviar uma vez");
        assert!(requests[0].contains("\"max_tokens\":16"));
        assert!(!requests[0].contains("max_completion_tokens"));
        assert!(requests[1].contains("max_completion_tokens"));
        assert!(!requests[1].contains("\"max_tokens\":"));
    }

    #[test]
    fn retries_after_rejected_temperature_value() {
        use crate::adapters::ai::custom_chat_bot_provider::CustomChatBotProvider;
        use crate::application::test_support::spawn_openai_sequence_mock;
        use crate::domain::settings::ProviderKind;
        use crate::ports::chat_bot_provider::ChatBotProvider;

        let (base_url, seen) = spawn_openai_sequence_mock(vec![
            (400, TEMPERATURE_VALUE_400.to_string()),
            (
                200,
                r#"{"choices":[{"message":{"content":"ok"}}]}"#.to_string(),
            ),
        ]);
        let provider = CustomChatBotProvider::new(ProviderConfig {
            kind: ProviderKind::Custom,
            model: "meu-modelo-local".into(),
            base_url: Some(base_url),
            api_key: "test-key".into(),
        });

        let request = CompletionRequest::user("oi").with_temperature(0.3);
        assert_eq!(provider.complete(&request).expect("complete").text, "ok");

        let requests = seen.lock().expect("lock").clone();
        assert_eq!(requests.len(), 2, "deve reenviar sem temperature");
        assert!(requests[0].contains("\"temperature\":0.3"));
        assert!(!requests[1].contains("temperature"));
    }

    #[test]
    fn keeps_temperature_for_models_that_support_it() {
        // Ensina o contrário: gpt-4o mantém `temperature` no primeiro envio.
        let config = ProviderConfig {
            kind: crate::domain::settings::ProviderKind::OpenAi,
            model: "gpt-4o".into(),
            base_url: None,
            api_key: "k".into(),
        };
        let request = CompletionRequest::user("oi");
        let body = build_body(&config, &request);
        assert!(body.get("temperature").is_none());

        let request = CompletionRequest::user("oi").with_temperature(0.0);
        let body = build_body(&config, &request);
        assert_eq!(body["temperature"], json!(0.0));
    }

    #[test]
    fn extracts_text_from_content_parts_and_legacy_field() {
        let parts = json!({"choices":[{"message":{"content":[
            {"type":"text","text":"a"},
            {"type":"text","text":"b"}
        ]}}]});
        assert_eq!(extract_text(&parts), "ab");

        let legacy = json!({"choices":[{"text":"legado"}]});
        assert_eq!(extract_text(&legacy), "legado");

        let empty = json!({"choices":[]});
        assert_eq!(extract_text(&empty), "");
    }

    #[test]
    fn grows_token_budget_up_to_the_cap() {
        let mut body = json!({"max_tokens": 100});
        assert!(grow_token_budget(&mut body));
        assert_eq!(body["max_tokens"], json!(200));

        body["max_tokens"] = json!(9_000);
        assert!(grow_token_budget(&mut body));
        assert_eq!(body["max_tokens"], json!(16_000));

        let mut completion_tokens = json!({"max_completion_tokens": 50});
        assert!(grow_token_budget(&mut completion_tokens));
        assert_eq!(completion_tokens["max_completion_tokens"], json!(100));

        let mut none = json!({"model": "m"});
        assert!(!grow_token_budget(&mut none));
    }

    #[test]
    fn empty_response_messages_are_actionable() {
        let refused = ParsedCompletion {
            text: String::new(),
            finish_reason: Some("stop".into()),
            refusal: Some("não posso".into()),
        };
        assert!(empty_response_message(&refused).contains("recusou"));

        let length = ParsedCompletion {
            text: String::new(),
            finish_reason: Some("length".into()),
            refusal: None,
        };
        assert!(empty_response_message(&length).contains("limite de tokens"));

        let filtered = ParsedCompletion {
            text: String::new(),
            finish_reason: Some("content_filter".into()),
            refusal: None,
        };
        assert!(empty_response_message(&filtered).contains("filtro de conteúdo"));

        let unknown = ParsedCompletion {
            text: String::new(),
            finish_reason: None,
            refusal: None,
        };
        assert_eq!(
            empty_response_message(&unknown),
            "provider retornou uma resposta vazia"
        );
    }

    /// Servidor mínimo: responde em sequência cada corpo da lista e encerra.
    fn serve_bodies(bodies: Vec<&'static str>) -> std::net::SocketAddr {
        use std::io::Write;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            for body in bodies {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut discard = [0u8; 4096];
                let _ = std::io::Read::read(&mut stream, &mut discard);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        addr
    }

    #[test]
    fn retries_once_when_the_provider_returns_an_empty_response() {
        let addr = serve_bodies(vec![
            r#"{"choices":[{"message":{"content":""},"finish_reason":"length"}]}"#,
            r#"{"choices":[{"message":{"content":"resumo ok"},"finish_reason":"stop"}]}"#,
        ]);
        let config = ProviderConfig {
            kind: crate::domain::settings::ProviderKind::OpenAi,
            model: "m".into(),
            base_url: None,
            api_key: "k".into(),
        };
        let completion = chat_completion(
            &config,
            &format!("http://{addr}"),
            &CompletionRequest::user("resuma").with_max_tokens(100),
            &[],
        )
        .expect("reenvio");
        assert_eq!(completion.text, "resumo ok");
    }

    #[test]
    fn empty_after_retry_reports_the_finish_reason() {
        let addr = serve_bodies(vec![
            r#"{"choices":[{"message":{"content":""},"finish_reason":"length"}]}"#,
            r#"{"choices":[{"message":{"content":""},"finish_reason":"length"}]}"#,
        ]);
        let config = ProviderConfig {
            kind: crate::domain::settings::ProviderKind::OpenAi,
            model: "m".into(),
            base_url: None,
            api_key: "k".into(),
        };
        let error = chat_completion(
            &config,
            &format!("http://{addr}"),
            &CompletionRequest::user("resuma").with_max_tokens(100),
            &[],
        )
        .expect_err("vazio");
        assert!(error.to_string().contains("finish_reason=length"));
    }
}
