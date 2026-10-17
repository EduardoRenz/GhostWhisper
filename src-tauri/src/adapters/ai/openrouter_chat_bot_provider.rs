//! Provider OpenRouter (API compatível com OpenAI).

use crate::domain::llm::{Completion, CompletionRequest, ProviderConfig};
use crate::error::Result;
use crate::ports::chat_bot_provider::ChatBotProvider;

use super::openai_compatible::chat_completion;

const BASE_URL: &str = "https://openrouter.ai/api/v1";

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
        // Cabeçalhos opcionais de atribuição recomendados pelo OpenRouter.
        let headers = [
            ("HTTP-Referer", "https://github.com/ghostwhisper"),
            ("X-Title", "GhostWhisper"),
        ];
        chat_completion(&self.config, BASE_URL, request, &headers)
    }
}
