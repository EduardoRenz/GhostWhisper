//! Caso de uso: resolver a configuração e chamar o provider de LLM.

use std::sync::Arc;

use crate::domain::llm::{Completion, CompletionRequest, ProviderConfig};
use crate::error::{GhostWhisperError, Result};
use crate::ports::chat_bot_provider::ChatBotProvider;
use crate::ports::llm_controller::LlmController;
use crate::ports::llm_provider_factory::ChatBotProviderFactory;
use crate::ports::secret_store::SecretStore;
use crate::ports::settings_store::SettingsStore;

/// Prompt trivial usado no teste de conexão.
const VALIDATION_PROMPT: &str = "Responda apenas com a palavra: ok";

/// Roteia completudes para o provider configurado pelo usuário.
pub struct LlmRouter {
    factory: Arc<dyn ChatBotProviderFactory>,
    settings: Arc<dyn SettingsStore>,
    secrets: Arc<dyn SecretStore>,
}

impl LlmRouter {
    /// Cria o roteador com as portas resolvidas.
    pub fn new(
        factory: Arc<dyn ChatBotProviderFactory>,
        settings: Arc<dyn SettingsStore>,
        secrets: Arc<dyn SecretStore>,
    ) -> Self {
        Self {
            factory,
            settings,
            secrets,
        }
    }

    /// Resolve a configuração ativa (com a chave em memória).
    pub fn resolve_config(&self) -> Result<ProviderConfig> {
        let settings = self.settings.load()?;
        settings.validate()?;
        let key = self
            .secrets
            .get(&settings.provider.secret_key())?
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                GhostWhisperError::Config(format!(
                    "nenhuma chave configurada para {}",
                    settings.provider.as_str()
                ))
            })?;

        Ok(ProviderConfig {
            kind: settings.provider,
            model: settings.model.clone(),
            base_url: settings.effective_base_url(),
            api_key: key,
        })
    }

    fn provider(&self) -> Result<Arc<dyn ChatBotProvider>> {
        self.factory.create(&self.resolve_config()?)
    }
}

impl LlmController for LlmRouter {
    fn provider_label(&self) -> Result<String> {
        Ok(self.provider()?.id())
    }

    fn validate(&self) -> Result<String> {
        let provider = self.provider()?;
        let request = CompletionRequest::user(VALIDATION_PROMPT)
            .with_system("Você é um teste de conectividade. Seja breve.")
            .with_max_tokens(16)
            .with_temperature(0.0);
        let completion = provider.complete(&request)?;
        Ok(completion.text)
    }

    fn complete(&self, request: &CompletionRequest) -> Result<Completion> {
        self.provider()?.complete(request)
    }

    fn complete_streaming(
        &self,
        request: &CompletionRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion> {
        self.provider()?.complete_streaming(request, on_text)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::adapters::ai::fake_chat_bot_provider::FakeChatBotProvider;
    use crate::adapters::secrets::in_memory_secret_store::InMemorySecretStore;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::domain::settings::{ProviderKind, Settings};

    struct CountingFactory {
        calls: Arc<AtomicUsize>,
    }

    impl ChatBotProviderFactory for CountingFactory {
        fn create(&self, _config: &ProviderConfig) -> Result<Arc<dyn ChatBotProvider>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(Arc::new(FakeChatBotProvider::new().with_response("ok")))
        }
    }

    fn router(settings: Settings, key: Option<&str>) -> (LlmRouter, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let secrets = Arc::new(InMemorySecretStore::new());
        if let Some(key) = key {
            secrets
                .set(&settings.provider.secret_key(), key)
                .expect("set key");
        }
        let router = LlmRouter::new(
            Arc::new(CountingFactory {
                calls: Arc::clone(&calls),
            }),
            Arc::new(InMemorySettingsStore::with_settings(settings)),
            secrets,
        );
        (router, calls)
    }

    #[test]
    fn validate_uses_configured_provider() {
        let (router, calls) = router(Settings::default(), Some("chave"));
        assert_eq!(router.validate().expect("validate"), "ok");
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn missing_key_is_reported() {
        let (router, _calls) = router(Settings::default(), None);
        let error = router.validate().unwrap_err();
        assert!(error.to_string().contains("nenhuma chave"));
    }

    #[test]
    fn custom_without_base_url_fails() {
        let settings = Settings {
            provider: ProviderKind::Custom,
            model: "llama".into(),
            base_url: None,
            ..Settings::default()
        };
        let (router, _calls) = router(settings, Some("chave"));
        assert!(router.validate().is_err());
    }

    #[test]
    fn provider_label_has_no_secret() {
        let (router, _calls) = router(Settings::default(), Some("super-secreta"));
        let label = router.provider_label().expect("label");
        assert!(!label.is_empty());
        assert!(!label.contains("super-secreta"));
    }
}
