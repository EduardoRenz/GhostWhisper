//! Suporte a endpoints compatíveis com a API de chat da OpenAI.
//!
//! Usado por `OpenAiChatBotProvider`, `OpenRouterChatBotProvider` e
//! `CustomChatBotProvider`. Nunca registra a chave de API em logs/erros.

use std::time::Duration;

use serde_json::{json, Value};

use crate::domain::llm::{Completion, CompletionRequest, ProviderConfig};
use crate::error::{GhostWhisperError, Result};

/// Tempo limite padrão das chamadas (segundos).
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Quantas vezes reenviamos após o provider rejeitar um parâmetro.
const MAX_PARAM_RETRIES: usize = 3;

/// Executa `POST {base_url}/chat/completions`.
///
/// Modelos novos rejeitam `max_tokens` (pedem `max_completion_tokens`) e alguns
/// rejeitam `temperature`. Ao receber um `400 unsupported_parameter`, o
/// parâmetro é renomeado/removido e a chamada é reenviada.
pub fn chat_completion(
    config: &ProviderConfig,
    base_url: &str,
    request: &CompletionRequest,
    extra_headers: &[(&str, &str)],
) -> Result<Completion> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let mut body = build_body(config, request);
    let mut retries = 0usize;

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
                let text = extract_text(&value);
                if text.is_empty() {
                    return Err(GhostWhisperError::Other(
                        "provider retornou uma resposta vazia".to_string(),
                    ));
                }
                return Ok(Completion {
                    text,
                    model: config.model.clone(),
                });
            }
            Err(ureq::Error::Status(code, response))
                if code == 400 && retries < MAX_PARAM_RETRIES =>
            {
                let detail = response.into_string().unwrap_or_default();
                let Some(param) = unsupported_param(&detail) else {
                    return Err(GhostWhisperError::Other(format!(
                        "provider respondeu {code}: {}",
                        truncate(&detail, 300)
                    )));
                };
                if !apply_unsupported_param_fallback(&mut body, &param) {
                    return Err(GhostWhisperError::Other(format!(
                        "provider respondeu {code}: {}",
                        truncate(&detail, 300)
                    )));
                }
                retries += 1;
                tracing::debug!("provider rejeitou '{param}'; reenviando sem ele");
            }
            Err(other) => return Err(map_transport_error(other)),
        }
    }
}

/// Monta o corpo da requisição a partir do pedido.
fn build_body(config: &ProviderConfig, request: &CompletionRequest) -> Value {
    let mut messages: Vec<Value> = Vec::new();
    if let Some(system) = &request.system {
        messages.push(json!({ "role": "system", "content": system }));
    }
    messages.push(json!({ "role": "user", "content": request.prompt }));

    let mut body = json!({ "model": config.model, "messages": messages });
    if let Some(max_tokens) = request.max_tokens {
        body["max_tokens"] = json!(max_tokens);
    }
    if let Some(temperature) = request.temperature {
        body["temperature"] = json!(temperature);
    }
    body
}

/// Extrai o texto da primeira escolha da resposta.
fn extract_text(value: &Value) -> String {
    value["choices"]
        .get(0)
        .and_then(|choice| choice["message"]["content"].as_str())
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Descobre qual parâmetro o provider recusou, se a falha for desse tipo.
fn unsupported_param(detail: &str) -> Option<String> {
    let value: Value = serde_json::from_str(detail).ok()?;
    let code = value["error"]["code"].as_str().unwrap_or_default();
    let message = value["error"]["message"].as_str().unwrap_or_default();
    let is_unsupported = code == "unsupported_parameter"
        || message.contains("Unsupported parameter")
        || message.contains("unsupported_parameter");
    if !is_unsupported {
        return None;
    }
    if let Some(param) = value["error"]["param"].as_str() {
        if !param.is_empty() {
            return Some(param.to_string());
        }
    }
    let marker = "Unsupported parameter: '";
    let start = message.find(marker)? + marker.len();
    let rest = &message[start..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// Ajusta o corpo: renomeia `max_tokens` e remove os demais recusados.
///
/// Retorna `false` quando o parâmetro não estava presente (nada a ajustar).
fn apply_unsupported_param_fallback(body: &mut Value, param: &str) -> bool {
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
            GhostWhisperError::Other(format!("provider respondeu {code}: {detail}"))
        }
        other => GhostWhisperError::Other(format!("falha de rede ao chamar o provider: {other}")),
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

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_TOKENS_400: &str = r#"{"error":{"message":"Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead.","type":"invalid_request_error","param":"max_tokens","code":"unsupported_parameter"}}"#;

    fn body_with_max_tokens() -> Value {
        json!({ "model": "m", "messages": [], "max_tokens": 16, "temperature": 0.0 })
    }

    #[test]
    fn detects_unsupported_param_from_code_and_field() {
        assert_eq!(
            unsupported_param(MAX_TOKENS_400).as_deref(),
            Some("max_tokens")
        );
    }

    #[test]
    fn detects_unsupported_param_from_message_only() {
        let detail =
            r#"{"error":{"message":"Unsupported parameter: 'temperature' is not supported."}}"#;
        assert_eq!(unsupported_param(detail).as_deref(), Some("temperature"));
    }

    #[test]
    fn ignores_unrelated_errors() {
        let detail = r#"{"error":{"message":"The model 'x' does not exist","param":"model","code":"model_not_found"}}"#;
        assert_eq!(unsupported_param(detail), None);
    }

    #[test]
    fn renames_max_tokens_to_max_completion_tokens() {
        let mut body = body_with_max_tokens();
        assert!(apply_unsupported_param_fallback(&mut body, "max_tokens"));
        assert!(body.get("max_tokens").is_none());
        assert_eq!(body["max_completion_tokens"], json!(16));
        // Idempotente: repetir não muda nada nem falha.
        assert!(apply_unsupported_param_fallback(&mut body, "max_tokens"));
        assert_eq!(body["max_completion_tokens"], json!(16));
    }

    #[test]
    fn removes_other_unsupported_params() {
        let mut body = body_with_max_tokens();
        assert!(apply_unsupported_param_fallback(&mut body, "temperature"));
        assert!(body.get("temperature").is_none());
        // Parâmetro ausente → nada a ajustar.
        assert!(!apply_unsupported_param_fallback(&mut body, "temperature"));
    }

    #[test]
    fn extracts_first_choice_text() {
        let value = json!({"choices":[{"message":{"content":"  ok  "}}]});
        assert_eq!(extract_text(&value), "ok");
        assert_eq!(extract_text(&json!({})), "");
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
}
