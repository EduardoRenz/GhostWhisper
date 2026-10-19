//! Provider OpenRouter (API compatível com OpenAI).

use crate::domain::llm::{Completion, CompletionRequest, ProviderConfig};
use crate::error::Result;
use crate::ports::chat_bot_provider::ChatBotProvider;

use super::openai_compatible::{chat_completion, chat_completion_streaming};

const BASE_URL: &str = "https://openrouter.ai/api/v1";

/// Cabeçalhos opcionais de atribuição recomendados pelo OpenRouter.
const ATTRIBUTION_HEADERS: [(&str, &str); 2] = [
    ("HTTP-Referer", "https://github.com/ghostwhisper"),
    ("X-Title", "GhostWhisper"),
];

/// Provider OpenRouter (agrega vários modelos).
pub struct OpenRouterChatBotProvider {
    config: ProviderConfig,
}

impl OpenRouterChatBotProvider {
    /// Cria o provider com a configuração resolvida.
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

impl ChatBotProvider for OpenRouterChatBotProvider {
    fn id(&self) -> String {
        format!("openrouter:{}", self.config.model)
    }

    fn complete(&self, request: &CompletionRequest) -> Result<Completion> {
        chat_completion(&self.config, BASE_URL, request, &ATTRIBUTION_HEADERS)
    }

    fn complete_streaming(
        &self,
        request: &CompletionRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion> {
        chat_completion_streaming(
            &self.config,
            BASE_URL,
            request,
            &ATTRIBUTION_HEADERS,
            on_text,
        )
    }
}
