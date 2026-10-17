//! Provider Google Gemini (`generateContent`).

use std::time::Duration;

use serde_json::{json, Value};

use crate::domain::llm::{Completion, CompletionRequest, ProviderConfig};
use crate::error::{GhostWhisperError, Result};
use crate::ports::chat_bot_provider::ChatBotProvider;

use super::openai_compatible::{map_transport_error, DEFAULT_TIMEOUT_SECS};

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Provider Google Gemini.
pub struct GeminiChatBotProvider {
    config: ProviderConfig,
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

        // A chave vai em cabeçalho (nunca na URL/query) para não vazar em logs.
        let response = ureq::post(&url)
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .set("Content-Type", "application/json")
            .set("x-goog-api-key", &self.config.api_key)
            .send_json(body)
            .map_err(map_transport_error)?;

        let value: Value = response
            .into_json()
            .map_err(|e| GhostWhisperError::Other(format!("resposta inválida do provider: {e}")))?;

        let text = value["candidates"]
            .get(0)
            .and_then(|candidate| candidate["content"]["parts"].as_array())
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|part| part["text"].as_str())
                    .collect::<String>()
            })
            .unwrap_or_default()
            .trim()
            .to_string();

        if text.is_empty() {
            return Err(GhostWhisperError::Other(
                "provider retornou uma resposta vazia".to_string(),
            ));
        }

        Ok(Completion {
            text,
            model: self.config.model.clone(),
        })
    }
}
