//! Fábrica padrão: escolhe o adaptador de provider pela configuração.

use std::sync::Arc;

use crate::domain::llm::ProviderConfig;
use crate::domain::settings::ProviderKind;
use crate::error::{GhostWhisperError, Result};
use crate::ports::chat_bot_provider::ChatBotProvider;
use crate::ports::llm_provider_factory::ChatBotProviderFactory;

use super::custom_chat_bot_provider::CustomChatBotProvider;
use super::gemini_chat_bot_provider::GeminiChatBotProvider;
use super::openai_chat_bot_provider::OpenAiChatBotProvider;
use super::openrouter_chat_bot_provider::OpenRouterChatBotProvider;

/// Único ponto que conhece todos os adaptadores de provider.
pub struct DefaultChatBotProviderFactory;

impl ChatBotProviderFactory for DefaultChatBotProviderFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Arc<dyn ChatBotProvider>> {
        if config.model.trim().is_empty() {
            return Err(GhostWhisperError::Config(
                "modelo não pode ser vazio".into(),
            ));
        }
        if config.api_key.trim().is_empty() {
            return Err(GhostWhisperError::Config(
                "chave de API ausente para o provider".into(),
            ));
        }

        let provider: Arc<dyn ChatBotProvider> = match config.kind {
            ProviderKind::OpenAi => Arc::new(OpenAiChatBotProvider::new(config.clone())),
            ProviderKind::Gemini => Arc::new(GeminiChatBotProvider::new(config.clone())),
            ProviderKind::OpenRouter => Arc::new(OpenRouterChatBotProvider::new(config.clone())),
            ProviderKind::Custom => Arc::new(CustomChatBotProvider::new(config.clone())),
        };
        Ok(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(kind: ProviderKind) -> ProviderConfig {
        ProviderConfig {
            kind,
            model: "modelo".into(),
            base_url: Some("https://exemplo.test/v1".into()),
            api_key: "chave".into(),
        }
    }

    #[test]
    fn builds_all_provider_kinds() {
        let factory = DefaultChatBotProviderFactory;
        assert!(factory.create(&config(ProviderKind::OpenAi)).is_ok());
        assert!(factory.create(&config(ProviderKind::Gemini)).is_ok());
        assert!(factory.create(&config(ProviderKind::OpenRouter)).is_ok());
        assert!(factory.create(&config(ProviderKind::Custom)).is_ok());
    }

    #[test]
    fn rejects_missing_key_or_model() {
        let factory = DefaultChatBotProviderFactory;
        let mut cfg = config(ProviderKind::OpenAi);
        cfg.api_key = "  ".into();
        assert!(factory.create(&cfg).is_err());

        let mut cfg = config(ProviderKind::OpenAi);
        cfg.model = "".into();
        assert!(factory.create(&cfg).is_err());
    }
}
