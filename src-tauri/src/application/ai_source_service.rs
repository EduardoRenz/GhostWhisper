//! Caso de uso: gerenciar as fontes de IA (BR-044).
//!
//! Junta o `SettingsStore` (a lista e a **ordem** das fontes), o
//! `SecretStore` (a chave de cada fonte) e o registro de saúde (o circuit
//! breaker — BR-045).

use std::sync::Arc;

use crate::domain::ai_route::AiRouteScope;
use crate::domain::ai_source::AiSource;
use crate::domain::ai_source::AiSourceHealth;
use crate::domain::llm::{CompletionRequest, ProviderConfig};
use crate::domain::settings::redact_secret;
use crate::error::{GhostWhisperError, Result};
use crate::ports::ai_source_service::AiSourceService;
use crate::ports::clock::Clock;
use crate::ports::event_publisher::{EventPublisher, LlmSourceUsage};
use crate::ports::llm_provider_factory::ChatBotProviderFactory;
use crate::ports::secret_store::SecretStore;
use crate::ports::settings_store::SettingsStore;

use super::ai_source_health::AiSourceHealthRegistry;

/// Prompt trivial do teste de conexão.
const VALIDATION_PROMPT: &str = "Responda apenas com a palavra: ok";

/// Gerencia as fontes de IA do usuário.
pub struct LocalAiSourceService {
    settings: Arc<dyn SettingsStore>,
    secrets: Arc<dyn SecretStore>,
    factory: Arc<dyn ChatBotProviderFactory>,
    health: Arc<AiSourceHealthRegistry>,
    clock: Arc<dyn Clock>,
    publisher: Arc<dyn EventPublisher>,
}

impl LocalAiSourceService {
    /// Cria o serviço com o publicador de eventos (a UI acompanha o uso).
    pub fn with_publisher(
        settings: Arc<dyn SettingsStore>,
        secrets: Arc<dyn SecretStore>,
        factory: Arc<dyn ChatBotProviderFactory>,
        health: Arc<AiSourceHealthRegistry>,
        clock: Arc<dyn Clock>,
        publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            settings,
            secrets,
            factory,
            health,
            clock,
            publisher,
        }
    }

    /// Procura uma fonte pelo id nas settings atuais.
    #[allow(dead_code)]
    fn source(&self, source_id: &str) -> Result<AiSource> {
        let settings = self.settings.load()?;
        settings.source(source_id).cloned().ok_or_else(|| {
            GhostWhisperError::Config(format!("fonte de IA desconhecida: {source_id}"))
        })
    }

    /// Chave da fonte no keyring.
    ///
    /// Não exige que a fonte esteja salva: a chave é indexada pelo **id**, e o
    /// id nasce no rascunho da tela de Configurações (BR-044).
    fn secret_key(source_id: &str) -> String {
        crate::domain::ai_source::secret_key_for(source_id)
    }
}

impl AiSourceService for LocalAiSourceService {
    fn list(&self) -> Result<Vec<AiSource>> {
        Ok(self.settings.load()?.ai_sources)
    }

    fn health(&self) -> Vec<AiSourceHealth> {
        // Informe de **todas** as fontes cadastradas (mesmo as que nunca
        // foram usadas) + as fontes conhecidas que saíram da lista.
        let mut items: Vec<AiSourceHealth> = self
            .settings
            .load()
            .map(|settings| {
                settings
                    .ai_sources
                    .iter()
                    .map(|source| self.health.health_of(&source.id))
                    .collect()
            })
            .unwrap_or_default();
        for entry in self.health.snapshot() {
            if !items.iter().any(|item| item.source_id == entry.source_id) {
                items.push(entry);
            }
        }
        items
    }

    fn set_key(&self, source_id: &str, key: &str) -> Result<()> {
        self.secrets.set(&Self::secret_key(source_id), key)
    }

    fn has_key(&self, source_id: &str) -> Result<bool> {
        Ok(self
            .secrets
            .get(&Self::secret_key(source_id))?
            .is_some_and(|value| !value.trim().is_empty()))
    }

    fn clear_key(&self, source_id: &str) -> Result<()> {
        self.secrets.delete(&Self::secret_key(source_id))
    }

    fn key_preview(&self, source_id: &str) -> Result<String> {
        let masked = self
            .secrets
            .get(&Self::secret_key(source_id))?
            .map(|value| redact_secret(&value))
            .unwrap_or_default();
        Ok(masked)
    }

    fn validate(&self, source: &AiSource) -> Result<String> {
        let secret_key = Self::secret_key(&source.id);
        let key = self
            .secrets
            .get(&secret_key)?
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                GhostWhisperError::Config(format!(
                    "nenhuma chave configurada para {}",
                    source.display_label()
                ))
            })?;

        let config = ProviderConfig {
            kind: source.kind,
            model: source.model.clone(),
            base_url: source.effective_base_url(),
            api_key: key,
        };
        let provider = self.factory.create(&config)?;
        let request = CompletionRequest::user(VALIDATION_PROMPT)
            .with_system("Você é um teste de conectividade. Seja breve.")
            .with_max_tokens(16)
            .with_temperature(0.0);

        let started = self.clock.now_ms();
        match provider.complete(&request) {
            Ok(completion) => {
                let latency = (self.clock.now_ms() - started).max(0) as u64;
                self.health.record_success(&source.id, latency);
                self.health.mark_fallback(&source.id, false);
                tracing::debug!(
                    source = %source.display_label(),
                    provider = %source.kind.as_str(),
                    model = %source.model,
                    latency_ms = latency,
                    prompt_tokens = ?completion.usage.map(|usage| usage.prompt_tokens),
                    completion_tokens = ?completion.usage.map(|usage| usage.completion_tokens),
                    reasoning_tokens = ?completion.usage.and_then(|usage| usage.reasoning_tokens),
                    "llm: validação da fonte concluída"
                );
                // A validação também é um uso: a UI passa a mostrar a fonte como
                // conectada (o evento é o mesmo do roteador — BR-045).
                self.publisher.emit_llm_source_used(&LlmSourceUsage {
                    source_id: source.id.clone(),
                    label: source.display_label(),
                    provider: source.kind.as_str().to_string(),
                    model: source.model.clone(),
                    scope: AiRouteScope::Default.label(),
                    is_fallback: false,
                    latency_ms: latency,
                });
                Ok(completion.text)
            }
            Err(error) => {
                self.health.record_failure(&source.id, &error);
                tracing::debug!(
                    source = %source.display_label(),
                    provider = %source.kind.as_str(),
                    model = %source.model,
                    error = %error,
                    "llm: validação da fonte falhou"
                );
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::adapters::ai::fake_chat_bot_provider::FakeChatBotProvider;
    use crate::adapters::secrets::in_memory_secret_store::InMemorySecretStore;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::adapters::system::system_clock::MutableClock;
    use crate::domain::settings::{default_source, ProviderKind, Settings};

    struct CountingFactory {
        calls: Arc<AtomicUsize>,
    }

    impl ChatBotProviderFactory for CountingFactory {
        fn create(
            &self,
            _config: &ProviderConfig,
        ) -> Result<Arc<dyn crate::ports::chat_bot_provider::ChatBotProvider>> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(Arc::new(FakeChatBotProvider::new().with_response("ok")))
        }
    }

    fn service() -> (
        LocalAiSourceService,
        Arc<InMemorySecretStore>,
        Arc<AtomicUsize>,
    ) {
        let calls = Arc::new(AtomicUsize::new(0));
        let secrets = Arc::new(InMemorySecretStore::new());
        let service = LocalAiSourceService::with_publisher(
            Arc::new(InMemorySettingsStore::with_settings(Settings::default())),
            secrets.clone(),
            Arc::new(CountingFactory {
                calls: Arc::clone(&calls),
            }),
            Arc::new(AiSourceHealthRegistry::new(Arc::new(MutableClock::new(0)))),
            Arc::new(MutableClock::new(0)),
            Arc::new(crate::adapters::events::null_event_publisher::NullEventPublisher),
        );
        (service, secrets, calls)
    }

    #[test]
    fn lists_the_default_source() {
        let (service, _secrets, _calls) = service();
        let sources = service.list().expect("lista");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].kind, ProviderKind::OpenAi);
    }

    #[test]
    fn manages_the_key_per_source() {
        let (service, secrets, _calls) = service();
        let source = default_source();
        assert!(!service.has_key(&source.id).expect("sem chave"));
        assert_eq!(service.key_preview(&source.id).expect("preview"), "");

        service
            .set_key(&source.id, "sk-secreta-1234")
            .expect("grava");
        assert!(service.has_key(&source.id).expect("com chave"));
        // A chave viva fica no keyring, sob a chave da **fonte**.
        assert_eq!(
            secrets.get(&source.secret_key()).expect("lê").as_deref(),
            Some("sk-secreta-1234")
        );
        // A prévia é mascarada.
        assert_eq!(
            service.key_preview(&source.id).expect("preview"),
            "••••••••·1234"
        );

        service.clear_key(&source.id).expect("limpa");
        assert!(!service.has_key(&source.id).expect("sem chave"));
    }

    #[test]
    fn the_key_of_a_draft_source_can_be_managed() {
        // A fonte ainda **não** está salva (está no rascunho da tela): a chave
        // vive no keyring, indexada pelo id — não pode depender das settings.
        let (service, secrets, _calls) = service();
        let mut draft = default_source();
        draft.id = "rascunho-1".into();
        draft.kind = ProviderKind::DeepSeek;

        service.set_key(&draft.id, "sk-draft").expect("grava");
        assert!(service.has_key(&draft.id).expect("tem chave"));
        assert_eq!(
            secrets.get(&draft.secret_key()).expect("lê").as_deref(),
            Some("sk-draft")
        );
        assert_eq!(
            service.key_preview(&draft.id).expect("preview"),
            "••••••••·raft"
        );
    }

    #[test]
    fn validate_needs_a_key() {
        let (service, _secrets, _calls) = service();
        let error = service.validate(&default_source()).unwrap_err();
        assert!(error.to_string().contains("nenhuma chave"));
    }

    #[test]
    fn validate_records_the_latency() {
        let (service, _secrets, calls) = service();
        let source = default_source();
        service.set_key(&source.id, "chave").expect("grava");
        assert_eq!(service.validate(&source).expect("ok"), "ok");
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        let health = service.health();
        let entry = health
            .iter()
            .find(|entry| entry.source_id == source.id)
            .expect("saúde");
        assert_eq!(entry.state, crate::domain::ai_source::AiSourceState::Ready);
    }

    #[test]
    fn health_covers_every_registered_source() {
        let (service, _secrets, _calls) = service();
        let health = service.health();
        assert_eq!(health.len(), 1);
        assert_eq!(health[0].source_id, default_source().id);
    }
}
