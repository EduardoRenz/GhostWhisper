//! Caso de uso: traduzir um trecho para o idioma de destino configurado.

use std::sync::Arc;

use crate::domain::ai_route::{AiCapability, AiRouteScope};
use crate::domain::llm::CompletionRequest;
use crate::error::{GhostWhisperError, Result};
use crate::ports::llm_controller::LlmController;
use crate::ports::settings_store::SettingsStore;
use crate::ports::translator::Translator;

use super::prompts;

/// Traduz textos usando o provider configurado (BYO key).
pub struct TranslationService {
    llm: Arc<dyn LlmController>,
    settings: Arc<dyn SettingsStore>,
}

impl TranslationService {
    /// Cria o serviço com as portas resolvidas.
    pub fn new(llm: Arc<dyn LlmController>, settings: Arc<dyn SettingsStore>) -> Self {
        Self { llm, settings }
    }
}

impl Translator for TranslationService {
    fn translate(&self, text: &str) -> Result<String> {
        let text = text.trim();
        if text.is_empty() {
            return Err(GhostWhisperError::Config(
                "texto vazio não pode ser traduzido".to_string(),
            ));
        }

        let settings = self.settings.load()?;
        let target = settings.target_language.trim();
        // Destino "auto" ou igual à origem = não traduzir.
        if target == "auto" || target == settings.source_language.trim() {
            return Ok(text.to_string());
        }

        let system = prompts::translation_system_prompt(target);
        let request = CompletionRequest::user(prompts::translation_user_prompt(text))
            .with_system(system)
            .with_temperature(0.0)
            .with_max_tokens(1024);

        Ok(self
            .llm
            .complete_for(
                &AiRouteScope::Capability(AiCapability::Translation),
                &request,
            )?
            .text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::application::test_support::FakeLlmController;
    use crate::domain::settings::Settings;

    fn service(settings: Settings) -> (TranslationService, Arc<FakeLlmController>) {
        let llm = Arc::new(FakeLlmController::new().with_response("hello"));
        let service = TranslationService::new(
            llm.clone(),
            Arc::new(InMemorySettingsStore::with_settings(settings)),
        );
        (service, llm)
    }

    #[test]
    fn translates_with_target_language_prompt() {
        let settings = Settings {
            target_language: "en".into(),
            ..Settings::default()
        };
        let (service, llm) = service(settings);

        assert_eq!(service.translate("bom dia").expect("translate"), "hello");

        let request = llm.last_request().expect("request");
        assert!(request.prompt.contains("bom dia"));
        let system = request.system.expect("system");
        assert!(system.contains("inglês"));
    }

    #[test]
    fn asks_the_llm_with_the_translation_capability() {
        // BR-048: a tradução pede a fonte da **capacidade** Translation.
        let settings = Settings {
            target_language: "en".into(),
            ..Settings::default()
        };
        let (service, llm) = service(settings);

        service.translate("bom dia").expect("translate");

        let scopes = llm.scopes();
        assert_eq!(
            scopes,
            vec![AiRouteScope::Capability(AiCapability::Translation)]
        );
    }

    #[test]
    fn skips_translation_when_target_is_auto() {
        let settings = Settings {
            target_language: "auto".into(),
            ..Settings::default()
        };
        let (service, llm) = service(settings);
        assert_eq!(service.translate("bom dia").expect("translate"), "bom dia");
        // Nenhuma chamada ao provider.
        assert!(llm.last_request().is_none());
    }

    #[test]
    fn skips_translation_when_target_equals_source() {
        let settings = Settings {
            source_language: "pt".into(),
            target_language: "pt".into(),
            ..Settings::default()
        };
        let (service, llm) = service(settings);
        assert_eq!(service.translate("bom dia").expect("translate"), "bom dia");
        assert!(llm.last_request().is_none());
    }

    #[test]
    fn rejects_empty_text() {
        let (service, _llm) = service(Settings::default());
        assert!(service.translate("   ").is_err());
    }

    #[test]
    fn propagates_provider_error() {
        let llm = Arc::new(FakeLlmController::new().with_failure("provider respondeu 401"));
        let service = TranslationService::new(llm, Arc::new(InMemorySettingsStore::new()));
        let error = service.translate("oi").unwrap_err().to_string();
        assert!(error.contains("401"));
    }

    #[test]
    fn translates_through_local_openai_compatible_provider() {
        use crate::adapters::ai::default_chat_bot_provider_factory::DefaultChatBotProviderFactory;
        use crate::adapters::secrets::in_memory_secret_store::InMemorySecretStore;
        use crate::application::llm_router::LlmRouter;
        use crate::application::test_support::spawn_openai_mock;
        use crate::domain::ai_source::AiSource;
        use crate::domain::settings::ProviderKind;
        use crate::ports::secret_store::SecretStore;

        let (base_url, seen) =
            spawn_openai_mock("{\"choices\":[{\"message\":{\"content\":\"hello\"}}]}");
        let mut source = AiSource::new("fonte-local", ProviderKind::Custom);
        source.model = "local-model".into();
        source.base_url = Some(base_url);
        let settings = Settings {
            ai_sources: vec![source.clone()],
            ..Settings::default()
        };

        let store = Arc::new(InMemorySettingsStore::with_settings(settings.clone()));
        let secrets = Arc::new(InMemorySecretStore::new());
        secrets
            .set(&source.secret_key(), "test-key")
            .expect("set key");
        let llm = Arc::new(LlmRouter::new(
            Arc::new(DefaultChatBotProviderFactory),
            store.clone(),
            secrets,
        ));

        let service = TranslationService::new(llm, store);
        assert_eq!(service.translate("bom dia").expect("translate"), "hello");

        // A requisição real chegou ao endpoint custom no formato OpenAI.
        let request = seen.lock().expect("lock").clone();
        assert!(request.contains("POST /chat/completions"));
        assert!(request.contains("bom dia"));
        assert!(request.contains("Bearer test-key"));
    }
}
