//! Provider Google Gemini (`generateContent`).

use std::time::Duration;

use serde_json::{json, Value};

use crate::domain::llm::{Completion, CompletionRequest, CompletionUsage, ProviderConfig};
use crate::error::{GhostWhisperError, Result};
use crate::ports::chat_bot_provider::ChatBotProvider;

use super::openai_compatible::{map_transport_error, request_timeout};

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Quantas vezes reenviamos quando o Gemini responde sem conteúdo.
const MAX_EMPTY_RETRIES: usize = 1;
/// Pausa antes do reenvio (ms).
const EMPTY_RETRY_BACKOFF_MS: u64 = 300;
/// Teto do `maxOutputTokens` ao crescer para o reenvio.
const MAX_GROWN_OUTPUT_TOKENS: u64 = 16_000;

/// Texto, `finishReason` e eventual bloqueio do prompt na resposta do Gemini.
fn extract_candidate(value: &Value) -> (String, Option<String>, Option<String>) {
    let candidate = &value["candidates"][0];
    let text = candidate["content"]["parts"]
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part["text"].as_str())
                .collect::<String>()
        })
        .unwrap_or_default()
        .trim()
        .to_string();
    (
        text,
        candidate["finishReason"].as_str().map(str::to_string),
        value["promptFeedback"]["blockReason"]
            .as_str()
            .map(str::to_string),
    )
}

/// Dobra o `maxOutputTokens` do corpo (para o reenvio por resposta vazia).
fn grow_output_budget(body: &mut Value) -> bool {
    let Some(current) = body["generationConfig"]["maxOutputTokens"].as_u64() else {
        return false;
    };
    let grown = current.saturating_mul(2).min(MAX_GROWN_OUTPUT_TOKENS);
    if grown <= current {
        return false;
    }
    body["generationConfig"]["maxOutputTokens"] = json!(grown);
    true
}

/// Mensagem de erro acionável para uma resposta vazia do Gemini.
fn empty_response_message(finish_reason: Option<&str>, blocked: Option<&str>) -> String {
    if let Some(reason) = blocked {
        return format!("provider bloqueou o prompt ({reason})");
    }
    match finish_reason {
        Some("MAX_TOKENS") => "provider esgotou o limite de tokens antes de escrever \
            (finishReason=MAX_TOKENS) — aumente o limite de tokens"
            .to_string(),
        Some("SAFETY") | Some("RECITATION") | Some("PROHIBITED_CONTENT") => {
            format!(
                "provider bloqueou a resposta pelo filtro de conteúdo (finishReason={})",
                finish_reason.unwrap_or_default()
            )
        }
        Some(reason) if !reason.is_empty() => {
            format!("provider retornou uma resposta vazia (finishReason={reason})")
        }
        _ => "provider retornou uma resposta vazia".to_string(),
    }
}

/// Provider Google Gemini.
pub struct GeminiChatBotProvider {
    config: ProviderConfig,
}

/// Dados da imagem em base64 (o Gemini usa `inline_data`).
fn base64_data(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

impl GeminiChatBotProvider {
    /// Cria o provider com a configuração resolvida.
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

impl ChatBotProvider for GeminiChatBotProvider {
    fn id(&self) -> String {
        format!("gemini:{}", self.config.model)
    }

    fn complete(&self, request: &CompletionRequest) -> Result<Completion> {
        let url = format!("{BASE_URL}/models/{}:generateContent", self.config.model);

        let mut body = json!({
            "contents": [
                { "role": "user", "parts": [ { "text": request.prompt } ] }
            ]
        });
        if !request.images.is_empty() {
            // Multimodal: a imagem vai em `inline_data` (BR-056).
            let mut parts: Vec<Value> = vec![json!({ "text": request.prompt })];
            for image in &request.images {
                parts.push(json!({
                    "inline_data": {
                        "mime_type": image.mime,
                        "data": base64_data(&image.data)
                    }
                }));
            }
            body["contents"][0]["parts"] = json!(parts);
        }
        if let Some(system) = &request.system {
            body["systemInstruction"] = json!({ "parts": [ { "text": system } ] });
        }
        let mut generation_config = json!({});
        if let Some(max_tokens) = request.max_tokens {
            generation_config["maxOutputTokens"] = json!(max_tokens);
        }
        if let Some(temperature) = request.temperature {
            generation_config["temperature"] = json!(temperature);
        }
        if generation_config.as_object().is_some_and(|o| !o.is_empty()) {
            body["generationConfig"] = generation_config;
        }

        let mut empty_retries = 0usize;
        loop {
            // A chave vai em cabeçalho (nunca na URL/query) para não vazar em logs.
            let response = ureq::post(&url)
                .timeout(request_timeout(request))
                .set("Content-Type", "application/json")
                .set("x-goog-api-key", &self.config.api_key)
                .send_json(body.clone())
                .map_err(map_transport_error)?;

            let value: Value = response.into_json().map_err(|e| {
                GhostWhisperError::Other(format!("resposta inválida do provider: {e}"))
            })?;

            let (text, finish_reason, blocked) = extract_candidate(&value);
            if text.is_empty() {
                if empty_retries < MAX_EMPTY_RETRIES {
                    empty_retries += 1;
                    let grew = grow_output_budget(&mut body);
                    tracing::debug!(
                        finish_reason = ?finish_reason,
                        grew_output_budget = grew,
                        "Gemini respondeu sem texto; reenviando"
                    );
                    std::thread::sleep(Duration::from_millis(EMPTY_RETRY_BACKOFF_MS));
                    continue;
                }
                return Err(GhostWhisperError::Other(empty_response_message(
                    finish_reason.as_deref(),
                    blocked.as_deref(),
                )));
            }

            return Ok(Completion {
                text,
                model: self.config.model.clone(),
                provider: None,
                usage: parse_usage(&value),
            });
        }
    }
}

/// Lê o `usageMetadata` do Gemini (tokens + raciocínio).
///
/// O Gemini conta o raciocínio em `thoughtsTokenCount` (modelos "thinking");
/// quando não há raciocínio, o campo vem ausente.
fn parse_usage(value: &Value) -> Option<CompletionUsage> {
    let usage = value.get("usageMetadata")?;
    let prompt_tokens = usage
        .get("promptTokenCount")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let completion_tokens = usage
        .get("candidatesTokenCount")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let total_tokens = usage
        .get("totalTokenCount")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let reasoning_tokens = usage
        .get("thoughtsTokenCount")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_text_from_candidate_parts() {
        let value = json!({
            "candidates": [{
                "content": { "parts": [{ "text": "a" }, { "text": "b" }] },
                "finishReason": "STOP"
            }]
        });
        let (text, finish, blocked) = extract_candidate(&value);
        assert_eq!(text, "ab");
        assert_eq!(finish.as_deref(), Some("STOP"));
        assert_eq!(blocked, None);
    }

    #[test]
    fn reads_the_gemini_usage_with_thoughts() {
        let value = json!({
            "usageMetadata": {
                "promptTokenCount": 90,
                "candidatesTokenCount": 30,
                "totalTokenCount": 120,
                "thoughtsTokenCount": 18
            }
        });
        let usage = parse_usage(&value).expect("usage");
        assert_eq!(usage.prompt_tokens, 90);
        assert_eq!(usage.completion_tokens, 30);
        assert_eq!(usage.total_tokens, 120);
        // O Gemini conta o raciocínio em `thoughtsTokenCount`.
        assert_eq!(usage.reasoning_tokens, Some(18));
    }

    #[test]
    fn extracts_prompt_block_reason() {
        let value = json!({ "promptFeedback": { "blockReason": "SAFETY" } });
        let (text, _, blocked) = extract_candidate(&value);
        assert_eq!(text, "");
        assert_eq!(blocked.as_deref(), Some("SAFETY"));
    }

    #[test]
    fn grows_output_budget_up_to_the_cap() {
        let mut body = json!({ "generationConfig": { "maxOutputTokens": 100 } });
        assert!(grow_output_budget(&mut body));
        assert_eq!(body["generationConfig"]["maxOutputTokens"], json!(200));

        body["generationConfig"]["maxOutputTokens"] = json!(9_000);
        assert!(grow_output_budget(&mut body));
        assert_eq!(body["generationConfig"]["maxOutputTokens"], json!(16_000));

        let mut none = json!({ "contents": [] });
        assert!(!grow_output_budget(&mut none));
    }

    #[test]
    fn empty_messages_are_actionable() {
        assert!(empty_response_message(None, Some("SAFETY")).contains("bloqueou o prompt"));
        assert!(empty_response_message(Some("MAX_TOKENS"), None).contains("limite de tokens"));
        assert!(empty_response_message(Some("SAFETY"), None).contains("filtro de conteúdo"));
        assert_eq!(
            empty_response_message(None, None),
            "provider retornou uma resposta vazia"
        );
    }
}
