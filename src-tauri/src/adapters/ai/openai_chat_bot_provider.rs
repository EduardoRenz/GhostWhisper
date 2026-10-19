//! Provider OpenAI.

use crate::domain::llm::{Completion, CompletionRequest, ProviderConfig};
use crate::error::Result;
use crate::ports::chat_bot_provider::ChatBotProvider;

use super::openai_compatible::{chat_completion, chat_completion_streaming};

const BASE_URL: &str = "https://api.openai.com/v1";

/// Provider oficial da OpenAI.
pub struct OpenAiChatBotProvider {
    config: ProviderConfig,
}

impl OpenAiChatBotProvider {
    /// Cria o provider com a configuração resolvida.
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

impl ChatBotProvider for OpenAiChatBotProvider {
    fn id(&self) -> String {
        format!("openai:{}", self.config.model)
    }

    fn complete(&self, request: &CompletionRequest) -> Result<Completion> {
        chat_completion(&self.config, BASE_URL, request, &[])
    }

    fn complete_streaming(
        &self,
        request: &CompletionRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion> {
        chat_completion_streaming(&self.config, BASE_URL, request, &[], on_text)
    }
}
