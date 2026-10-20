//! Caso de uso: resolver a fonte de IA e chamar o provider.
//!
//! O roteador percorre uma **cadeia** de fontes (BR-044): a primeira da lista é
//! a primária; se ela falhar, a próxima habilitada assume (BR-045). Cada fonte
//! tem um **circuit breaker** — uma fonte com o circuito aberto é pulada sem
//! gastar timeout de rede.
//!
//! O escopo (BR-048) escolhe por onde a cadeia começa: uma rota de agente ou
//! de capacidade começa pela fonte vinculada; sem rota, começa pela primária.

use std::sync::Arc;

use crate::domain::ai_route::{resolve_route, AiRouteScope};
use crate::domain::ai_source::AiSource;
use crate::domain::llm::{Completion, CompletionRequest, CompletionUsage, ProviderConfig};
use crate::error::{GhostWhisperError, Result};
use crate::ports::chat_bot_provider::ChatBotProvider;
use crate::ports::clock::Clock;
use crate::ports::event_publisher::{EventPublisher, LlmSourceUsage};
use crate::ports::llm_controller::LlmController;
use crate::ports::llm_provider_factory::ChatBotProviderFactory;
use crate::ports::secret_store::SecretStore;
use crate::ports::settings_store::SettingsStore;

use super::ai_source_health::AiSourceHealthRegistry;

/// Prompt trivial usado no teste de conexão.
const VALIDATION_PROMPT: &str = "Responda apenas com a palavra: ok";

/// Dados do modelo usados na chamada — é o que vai para o log de debug
/// (BR-050).
///
/// Ficam num struct (e não no macro) para serem **testáveis** sem depender do
/// subscriber global do `tracing`.
#[derive(Debug, Clone, PartialEq)]
struct ModelCallInfo {
    /// Tarefa que chamou (agente, capacidade ou padrão).
    scope: String,
    /// Nome de exibição da fonte.
    source: String,
    /// Id da fonte.
    source_id: String,
    /// Provedor (`openai`, `deepseek`, …).
    provider: String,
    /// Modelo realmente usado.
    model: String,
    /// A fonte respondeu em fallback?
    is_fallback: bool,
    /// Teto de tokens do pedido.
    max_tokens: Option<u32>,
    /// Temperatura do pedido.
    temperature: Option<f32>,
    /// Latência da chamada (ms).
    latency_ms: u64,
    /// Uso de tokens reportado pelo provider.
    usage: Option<CompletionUsage>,
    /// Nível de raciocínio (ver [`reasoning_level`]).
    reasoning: &'static str,
}

impl ModelCallInfo {
    /// Monta os dados da chamada.
    fn new(
        source: &AiSource,
        scope: &AiRouteScope,
        request: &CompletionRequest,
        latency_ms: u64,
        is_fallback: bool,
        usage: Option<CompletionUsage>,
    ) -> Self {
        Self {
            scope: scope.label(),
            source: source.display_label(),
            source_id: source.id.clone(),
            provider: source.kind.as_str().to_string(),
            model: source.model.clone(),
            is_fallback,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            latency_ms,
            usage,
            reasoning: reasoning_level(source, usage.as_ref()),
        }
    }
}

/// Nível de raciocínio do modelo, para o log de debug.
///
/// O app **não** manda parâmetro de raciocínio: o nível vem do próprio modelo
/// (modelos "reasoning") e, quando o provider informa, dos tokens de
/// raciocínio consumidos.
fn reasoning_level(source: &AiSource, usage: Option<&CompletionUsage>) -> &'static str {
    let model = source.model.to_ascii_lowercase();
    let is_reasoning_model = model.starts_with("gpt-5")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
        || model.contains("reasoner")
        || model.contains("thinking");
    if usage.is_some_and(|usage| usage.has_reasoning()) {
        return "reasoning (tokens reportados)";
    }
    if is_reasoning_model {
        return "reasoning (modelo)";
    }
    "padrão"
}

/// Roteia completudes para a cadeia de fontes de IA.
pub struct LlmRouter {
    factory: Arc<dyn ChatBotProviderFactory>,
    settings: Arc<dyn SettingsStore>,
    secrets: Arc<dyn SecretStore>,
    health: Arc<AiSourceHealthRegistry>,
    clock: Arc<dyn Clock>,
    publisher: Arc<dyn EventPublisher>,
}

impl LlmRouter {
    /// Cria o roteador com as portas resolvidas.
    ///
    /// O registro de saúde e o publicador de eventos são os padrões (sem
    /// eventos). No composition root use [`with_telemetry`](Self::with_telemetry)
    /// para compartilhar o registro com o serviço de fontes.
    #[cfg(test)]
    pub fn new(
        factory: Arc<dyn ChatBotProviderFactory>,
        settings: Arc<dyn SettingsStore>,
        secrets: Arc<dyn SecretStore>,
    ) -> Self {
        Self::with_telemetry(
            factory,
            settings,
            secrets,
            Arc::new(AiSourceHealthRegistry::new(Arc::new(
                crate::adapters::system::system_clock::SystemClock,
            ))),
            Arc::new(crate::adapters::system::system_clock::SystemClock),
            Arc::new(crate::adapters::events::null_event_publisher::NullEventPublisher),
        )
    }

    /// Cria o roteador completo (registro de saúde, relógio e eventos).
    pub fn with_telemetry(
        factory: Arc<dyn ChatBotProviderFactory>,
        settings: Arc<dyn SettingsStore>,
        secrets: Arc<dyn SecretStore>,
        health: Arc<AiSourceHealthRegistry>,
        clock: Arc<dyn Clock>,
        publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            factory,
            settings,
            secrets,
            health,
            clock,
            publisher,
        }
    }

    /// Cadeia de candidatas para um escopo, na ordem de tentativa.
    ///
    /// A fonte da rota (quando existe) vem primeiro; as demais mantêm a ordem
    /// da lista. Fontes desabilitadas ficam de fora.
    fn candidates(&self, scope: &AiRouteScope) -> Result<Vec<AiSource>> {
        let settings = self.settings.load()?;
        let enabled: Vec<AiSource> = settings
            .ai_sources
            .iter()
            .filter(|source| source.enabled)
            .cloned()
            .collect();
        let routed = resolve_route(&settings.ai_routes, scope);

        let mut ordered: Vec<AiSource> = Vec::new();
        if let Some(source_id) = routed {
            if let Some(source) = enabled.iter().find(|source| source.id == source_id) {
                ordered.push(source.clone());
            }
        }
        for source in enabled {
            if !ordered.iter().any(|item| item.id == source.id) {
                ordered.push(source);
            }
        }
        Ok(ordered)
    }

    /// Resolve a configuração de uma fonte (com a chave em memória).
    fn resolve_config(&self, source: &AiSource) -> Result<ProviderConfig> {
        let key = self
            .secrets
            .get(&source.secret_key())?
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                GhostWhisperError::Config(format!(
                    "nenhuma chave configurada para {}",
                    source.display_label()
                ))
            })?;
        Ok(ProviderConfig {
            kind: source.kind,
            model: source.model.clone(),
            base_url: source.effective_base_url(),
            api_key: key,
        })
    }

    /// Provider de uma fonte (cria o adapter pelo factory).
    fn provider(&self, source: &AiSource) -> Result<Arc<dyn ChatBotProvider>> {
        self.factory.create(&self.resolve_config(source)?)
    }

    /// Registra o uso da fonte e publica o evento (BR-045).
    fn report_success(
        &self,
        source: &AiSource,
        latency_ms: u64,
        is_fallback: bool,
        scope: &AiRouteScope,
        request: &CompletionRequest,
        usage: Option<CompletionUsage>,
    ) {
        self.health.record_success(&source.id, latency_ms);
        self.health.mark_fallback(&source.id, is_fallback);
        let info = ModelCallInfo::new(source, scope, request, latency_ms, is_fallback, usage);
        tracing::debug!(
            scope = %info.scope,
            source = %info.source,
            source_id = %info.source_id,
            provider = %info.provider,
            model = %info.model,
            is_fallback = info.is_fallback,
            max_tokens = ?info.max_tokens,
            temperature = ?info.temperature,
            latency_ms = info.latency_ms,
            prompt_tokens = ?info.usage.map(|usage| usage.prompt_tokens),
            completion_tokens = ?info.usage.map(|usage| usage.completion_tokens),
            total_tokens = ?info.usage.map(|usage| usage.total_tokens),
            reasoning_tokens = ?info.usage.and_then(|usage| usage.reasoning_tokens),
            reasoning = info.reasoning,
            "llm: resposta concluída"
        );
        self.publisher.emit_llm_source_used(&LlmSourceUsage {
            source_id: source.id.clone(),
            label: source.display_label(),
            provider: source.kind.as_str().to_string(),
            model: source.model.clone(),
            scope: scope.label(),
            is_fallback,
            latency_ms,
        });
    }

    /// Registra a falha da fonte.
    fn report_failure(&self, source: &AiSource, error: &GhostWhisperError, scope: &AiRouteScope) {
        tracing::debug!(
            scope = %scope.label(),
            source = %source.display_label(),
            source_id = %source.id,
            provider = %source.kind.as_str(),
            model = %source.model,
            error = %error,
            "llm: chamada falhou"
        );
        self.health.record_failure(&source.id, error);
    }

    /// Mensagem agregada das falhas (sem segredos).
    fn aggregate(errors: &[(String, GhostWhisperError)]) -> GhostWhisperError {
        let detail = errors
            .iter()
            .map(|(label, error)| format!("{label}: {error}"))
            .collect::<Vec<_>>()
            .join(" · ");
        GhostWhisperError::Other(format!("nenhuma fonte de IA respondeu ({detail})"))
    }
}

impl LlmController for LlmRouter {
    fn provider_label(&self) -> Result<String> {
        let candidates = self.candidates(&AiRouteScope::Default)?;
        let source = candidates.first().ok_or_else(|| {
            GhostWhisperError::Config("nenhuma fonte de IA habilitada".to_string())
        })?;
        Ok(self.provider(source)?.id())
    }

    fn validate(&self) -> Result<String> {
        let candidates = self.candidates(&AiRouteScope::Default)?;
        let source = candidates.first().ok_or_else(|| {
            GhostWhisperError::Config("nenhuma fonte de IA habilitada".to_string())
        })?;
        let provider = self.provider(source)?;
        let request = CompletionRequest::user(VALIDATION_PROMPT)
            .with_system("Você é um teste de conectividade. Seja breve.")
            .with_max_tokens(16)
            .with_temperature(0.0);
        let started = self.clock.now_ms();
        match provider.complete(&request) {
            Ok(completion) => {
                let latency = (self.clock.now_ms() - started).max(0) as u64;
                self.report_success(
                    source,
                    latency,
                    false,
                    &AiRouteScope::Default,
                    &request,
                    completion.usage,
                );
                Ok(completion.text)
            }
            Err(error) => {
                self.report_failure(source, &error, &AiRouteScope::Default);
                Err(error)
            }
        }
    }

    fn complete(&self, request: &CompletionRequest) -> Result<Completion> {
        self.complete_for(&AiRouteScope::Default, request)
    }

    fn complete_for(
        &self,
        scope: &AiRouteScope,
        request: &CompletionRequest,
    ) -> Result<Completion> {
        let candidates = self.candidates(scope)?;
        if candidates.is_empty() {
            return Err(GhostWhisperError::Config(
                "nenhuma fonte de IA habilitada".to_string(),
            ));
        }

        let mut failures: Vec<(String, GhostWhisperError)> = Vec::new();
        for (index, source) in candidates.iter().enumerate() {
            // Circuito aberto: pula a fonte sem gastar timeout (BR-045).
            if self.health.is_open(&source.id) {
                continue;
            }
            let provider = match self.provider(source) {
                Ok(provider) => provider,
                Err(error) => {
                    failures.push((source.display_label(), error));
                    continue;
                }
            };
            let started = self.clock.now_ms();
            match provider.complete(request) {
                Ok(completion) => {
                    let latency = (self.clock.now_ms() - started).max(0) as u64;
                    self.report_success(
                        source,
                        latency,
                        index > 0,
                        scope,
                        request,
                        completion.usage,
                    );
                    return Ok(completion);
                }
                Err(error) => {
                    self.report_failure(source, &error, scope);
                    failures.push((source.display_label(), error));
                }
            }
        }

        Err(Self::aggregate(&failures))
    }

    fn complete_streaming(
        &self,
        request: &CompletionRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion> {
        self.complete_streaming_for(&AiRouteScope::Default, request, on_text)
    }

    fn complete_streaming_for(
        &self,
        scope: &AiRouteScope,
        request: &CompletionRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion> {
        let candidates = self.candidates(scope)?;
        if candidates.is_empty() {
            return Err(GhostWhisperError::Config(
                "nenhuma fonte de IA habilitada".to_string(),
            ));
        }

        let mut failures: Vec<(String, GhostWhisperError)> = Vec::new();
        for (index, source) in candidates.iter().enumerate() {
            if self.health.is_open(&source.id) {
                continue;
            }
            let provider = match self.provider(source) {
                Ok(provider) => provider,
                Err(error) => {
                    failures.push((source.display_label(), error));
                    continue;
                }
            };

            // Rastreia se a fonte já **emitiu** texto: depois disso, trocar de
            // fonte duplicaria o texto na UI.
            let mut emitted = false;
            let mut sink = |text: &str| {
                emitted = true;
                on_text(text);
            };

            let started = self.clock.now_ms();
            match provider.complete_streaming(request, &mut sink) {
                Ok(completion) => {
                    let latency = (self.clock.now_ms() - started).max(0) as u64;
                    self.report_success(
                        source,
                        latency,
                        index > 0,
                        scope,
                        request,
                        completion.usage,
                    );
                    return Ok(completion);
                }
                Err(error) => {
                    self.report_failure(source, &error, scope);
                    if emitted {
                        // A fonte já falou: o erro sobe (sem trocar de fonte).
                        return Err(error);
                    }
                    failures.push((source.display_label(), error));
                }
            }
        }

        Err(Self::aggregate(&failures))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use super::*;
    use crate::adapters::ai::fake_chat_bot_provider::FakeChatBotProvider;
    use crate::adapters::secrets::in_memory_secret_store::InMemorySecretStore;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::adapters::system::system_clock::MutableClock;
    use crate::domain::ai_route::AiRoute;
    use crate::domain::ai_source::AiSource;
    use crate::domain::settings::{ProviderKind, Settings};
    use crate::ports::event_publisher::SessionStateEvent;
    use crate::ports::session_controller::AudioChannel;
    use crate::ports::suggestion_generator::Suggestion;

    /// Provider que falha as `failures` primeiras vezes.
    struct FlakyProvider {
        failures: AtomicUsize,
        response: String,
    }

    impl FlakyProvider {
        fn new(failures: usize) -> Self {
            Self {
                failures: AtomicUsize::new(failures),
                response: "ok".to_string(),
            }
        }
    }

    impl ChatBotProvider for FlakyProvider {
        fn id(&self) -> String {
            "flaky".to_string()
        }

        fn complete(&self, _request: &CompletionRequest) -> Result<Completion> {
            if self
                .failures
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                    current.checked_sub(1)
                })
                .is_ok()
            {
                return Err(GhostWhisperError::Other("falhou".to_string()));
            }
            Ok(Completion {
                text: self.response.clone(),
                model: "flaky".to_string(),
                provider: None,
                usage: None,
            })
        }
    }

    /// Factory que decide o comportamento pelo `model` da configuração.
    struct ScriptedFactory {
        calls: Mutex<Vec<String>>,
    }

    impl ScriptedFactory {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl ChatBotProviderFactory for ScriptedFactory {
        fn create(&self, config: &ProviderConfig) -> Result<Arc<dyn ChatBotProvider>> {
            self.calls.lock().expect("calls").push(config.model.clone());
            if config.model == "sempre-falha" {
                return Ok(Arc::new(FlakyProvider::new(usize::MAX)));
            }
            if config.model == "falha-1" {
                return Ok(Arc::new(FlakyProvider::new(1)));
            }
            Ok(Arc::new(FakeChatBotProvider::new().with_response("ok")))
        }
    }

    /// Publicador que registra os usos das fontes.
    #[derive(Default)]
    struct RecordingPublisher {
        usages: Mutex<Vec<LlmSourceUsage>>,
    }

    impl EventPublisher for RecordingPublisher {
        fn emit_segment(&self, _segment: &crate::domain::transcription::TranscriptSegment) {}
        fn emit_audio_level(
            &self,
            _channel: AudioChannel,
            _level: crate::domain::audio::AudioLevel,
        ) {
        }
        fn emit_session_state(&self, _state: &SessionStateEvent) {}
        fn emit_insight(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion_cadence(
            &self,
            _cadence: &crate::ports::event_publisher::SuggestionCadenceEvent,
        ) {
        }
        fn emit_asr_status(&self, _status: &crate::domain::transcription::AsrStatus) {}
        fn emit_llm_source_used(&self, usage: &LlmSourceUsage) {
            self.usages.lock().expect("usages").push(usage.clone());
        }
        fn emit_journal(&self, _update: &crate::ports::event_publisher::JournalUpdate) {}
    }

    fn source(id: &str, kind: ProviderKind, model: &str) -> AiSource {
        let mut source = AiSource::new(id, kind);
        source.model = model.to_string();
        source
    }

    /// Atalho do fixture do roteador (evita um tipo gigante na assinatura).
    type RouterFixture = (
        LlmRouter,
        Arc<ScriptedFactory>,
        Arc<InMemorySecretStore>,
        Arc<RecordingPublisher>,
        Arc<AiSourceHealthRegistry>,
        Arc<MutableClock>,
    );

    fn router(settings: Settings, keys: &[(&str, &str)]) -> RouterFixture {
        let factory = Arc::new(ScriptedFactory::new());
        let secrets = Arc::new(InMemorySecretStore::new());
        for (id, key) in keys {
            secrets
                .set(&format!("llm.source.{id}"), key)
                .expect("set key");
        }
        let clock = Arc::new(MutableClock::new(0));
        let health = Arc::new(AiSourceHealthRegistry::new(clock.clone()));
        let publisher = Arc::new(RecordingPublisher::default());
        let router = LlmRouter::with_telemetry(
            factory.clone(),
            Arc::new(InMemorySettingsStore::with_settings(settings)),
            secrets.clone(),
            health.clone(),
            clock.clone(),
            publisher.clone(),
        );
        (router, factory, secrets, publisher, health, clock)
    }

    #[test]
    fn the_debug_log_carries_the_model_data_of_the_current_task() {
        // O log de debug precisa dizer **qual** modelo respondeu a tarefa atual,
        // com o nível de raciocínio quando existir.
        let source = source("a", ProviderKind::DeepSeek, "deepseek-reasoner");
        let request = CompletionRequest::user("oi")
            .with_max_tokens(420)
            .with_temperature(0.0);
        let usage = CompletionUsage {
            prompt_tokens: 1180,
            completion_tokens: 260,
            total_tokens: 1440,
            reasoning_tokens: Some(96),
        };

        let info = ModelCallInfo::new(
            &source,
            &AiRouteScope::Agent("meeting".into()),
            &request,
            1420,
            true,
            Some(usage),
        );

        assert_eq!(info.scope, "agent:meeting");
        assert_eq!(info.source, "DeepSeek");
        assert_eq!(info.source_id, "a");
        assert_eq!(info.provider, "deepseek");
        assert_eq!(info.model, "deepseek-reasoner");
        assert!(info.is_fallback);
        assert_eq!(info.max_tokens, Some(420));
        assert_eq!(info.temperature, Some(0.0));
        assert_eq!(info.latency_ms, 1420);
        assert_eq!(
            info.usage.and_then(|usage| usage.reasoning_tokens),
            Some(96)
        );
        // Tokens de raciocínio reportados: o nível vem deles.
        assert_eq!(info.reasoning, "reasoning (tokens reportados)");
    }

    #[test]
    fn the_scope_labels_identify_the_current_task() {
        assert_eq!(AiRouteScope::Default.label(), "default");
        assert_eq!(
            AiRouteScope::Capability(crate::domain::ai_route::AiCapability::Summary).label(),
            "capability:summary"
        );
        assert_eq!(
            AiRouteScope::Agent("meeting".into()).label(),
            "agent:meeting"
        );
    }

    #[test]
    fn the_event_carries_the_model_and_the_scope() {
        // A UI mostra **qual** modelo respondeu cada tarefa (BR-051): o evento
        // precisa trazer modelo, provider e escopo.
        let settings = Settings {
            ai_sources: vec![source("a", ProviderKind::DeepSeek, "deepseek-reasoner")],
            ..Settings::default()
        };
        let (router, _factory, _secrets, publisher, _health, _clock) =
            router(settings, &[("a", "chave")]);

        router
            .complete_for(
                &AiRouteScope::Agent("meeting".into()),
                &CompletionRequest::user("oi"),
            )
            .expect("ok");

        let usages = publisher.usages.lock().expect("usages").clone();
        assert_eq!(usages.len(), 1);
        let usage = &usages[0];
        assert_eq!(usage.source_id, "a");
        assert_eq!(usage.label, "DeepSeek");
        assert_eq!(usage.provider, "deepseek");
        assert_eq!(usage.model, "deepseek-reasoner");
        assert_eq!(usage.scope, "agent:meeting");
        assert!(!usage.is_fallback);
    }

    #[test]
    fn the_event_marks_the_fallback() {
        let settings = Settings {
            ai_sources: vec![
                source("a", ProviderKind::OpenAi, "sempre-falha"),
                source("b", ProviderKind::DeepSeek, "deepseek-chat"),
            ],
            ..Settings::default()
        };
        let (router, _factory, _secrets, publisher, _health, _clock) =
            router(settings, &[("a", "chave"), ("b", "chave")]);
        router
            .complete_for(
                &AiRouteScope::Capability(crate::domain::ai_route::AiCapability::Summary),
                &CompletionRequest::user("oi"),
            )
            .expect("ok");

        let usages = publisher.usages.lock().expect("usages").clone();
        assert_eq!(usages.len(), 1);
        assert!(usages[0].is_fallback);
        assert_eq!(usages[0].scope, "capability:summary");
        assert_eq!(usages[0].model, "deepseek-chat");
    }

    #[test]
    fn a_call_marks_the_source_as_used() {
        // O card da tela mostra "Nunca usada" até a fonte responder de fato:
        // o roteador precisa marcar a saúde da fonte que respondeu.
        let settings = Settings {
            ai_sources: vec![source("a", ProviderKind::DeepSeek, "deepseek-chat")],
            ..Settings::default()
        };
        let (router, _factory, _secrets, _publisher, health, _clock) =
            router(settings, &[("a", "chave")]);

        let before = health.health_of("a");
        assert_eq!(
            before.state,
            crate::domain::ai_source::AiSourceState::Unknown
        );

        router.complete(&CompletionRequest::user("oi")).expect("ok");

        let after = health.health_of("a");
        assert_eq!(after.state, crate::domain::ai_source::AiSourceState::Ready);
        assert!(after.latency_ms.is_some());
        assert!(!after.is_fallback);
    }

    #[test]
    fn a_reasoning_model_is_flagged_in_the_log() {
        let reasoning = source("a", ProviderKind::DeepSeek, "deepseek-reasoner");
        assert_eq!(reasoning_level(&reasoning, None), "reasoning (modelo)");

        let standard = source("b", ProviderKind::OpenAi, "gpt-4o");
        assert_eq!(reasoning_level(&standard, None), "padrão");

        // Quando o provider reporta tokens de raciocínio, o nível vem deles.
        let usage = CompletionUsage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
            reasoning_tokens: Some(12),
        };
        assert_eq!(
            reasoning_level(&standard, Some(&usage)),
            "reasoning (tokens reportados)"
        );
    }

    #[test]
    fn uses_the_primary_source() {
        let settings = Settings {
            ai_sources: vec![source("a", ProviderKind::OpenAi, "gpt-4o-mini")],
            ..Settings::default()
        };
        let (router, _factory, _secrets, _publisher, _health, _clock) =
            router(settings, &[("a", "chave")]);
        assert_eq!(
            router
                .complete(&CompletionRequest::user("oi"))
                .expect("ok")
                .text,
            "ok"
        );
    }

    #[test]
    fn falls_back_to_the_next_source() {
        let settings = Settings {
            ai_sources: vec![
                source("a", ProviderKind::OpenAi, "sempre-falha"),
                source("b", ProviderKind::DeepSeek, "deepseek-chat"),
            ],
            ..Settings::default()
        };
        let (router, factory, _secrets, publisher, _health, _clock) =
            router(settings, &[("a", "chave"), ("b", "chave")]);
        let completion = router.complete(&CompletionRequest::user("oi")).expect("ok");
        assert_eq!(completion.text, "ok");

        // As duas foram tentadas, na ordem.
        let calls = factory.calls.lock().expect("calls").clone();
        assert_eq!(calls, vec!["sempre-falha", "deepseek-chat"]);

        // O evento marca o fallback.
        let usages = publisher.usages.lock().expect("usages").clone();
        assert_eq!(usages.len(), 1);
        assert!(usages[0].is_fallback);
        assert_eq!(usages[0].source_id, "b");
    }

    #[test]
    fn a_source_without_a_key_is_skipped() {
        let settings = Settings {
            ai_sources: vec![
                source("a", ProviderKind::OpenAi, "gpt-4o-mini"),
                source("b", ProviderKind::DeepSeek, "deepseek-chat"),
            ],
            ..Settings::default()
        };
        let (router, factory, _secrets, _publisher, _health, _clock) =
            router(settings, &[("b", "chave")]);
        router.complete(&CompletionRequest::user("oi")).expect("ok");
        let calls = factory.calls.lock().expect("calls").clone();
        // A fonte "a" nem foi chamada (não tem chave).
        assert_eq!(calls, vec!["deepseek-chat"]);
    }

    #[test]
    fn all_sources_failing_reports_every_reason() {
        let settings = Settings {
            ai_sources: vec![
                source("a", ProviderKind::OpenAi, "sempre-falha"),
                source("b", ProviderKind::DeepSeek, "sempre-falha"),
            ],
            ..Settings::default()
        };
        let (router, _factory, _secrets, _publisher, _health, _clock) =
            router(settings, &[("a", "chave"), ("b", "chave")]);
        let error = router
            .complete(&CompletionRequest::user("oi"))
            .expect_err("falha");
        let message = error.to_string();
        assert!(message.contains("nenhuma fonte de IA respondeu"));
        assert!(message.contains("OpenAI"));
        assert!(message.contains("DeepSeek"));
        assert!(!message.contains("chave"));
    }

    #[test]
    fn an_open_circuit_skips_the_source() {
        let settings = Settings {
            ai_sources: vec![
                source("a", ProviderKind::OpenAi, "sempre-falha"),
                source("b", ProviderKind::DeepSeek, "deepseek-chat"),
            ],
            ..Settings::default()
        };
        let (router, factory, _secrets, _publisher, _health, _clock) =
            router(settings, &[("a", "chave"), ("b", "chave")]);

        // Três falhas abrem o circuito de "a".
        for _ in 0..3 {
            let _ = router.complete(&CompletionRequest::user("oi"));
        }
        let calls_before = factory.calls.lock().expect("calls").len();
        let _ = router.complete(&CompletionRequest::user("oi"));
        let calls_after = factory.calls.lock().expect("calls").len();
        // Só a fonte "b" foi chamada nesta rodada.
        assert_eq!(calls_after - calls_before, 1);
    }

    #[test]
    fn a_route_starts_the_chain_at_the_linked_source() {
        let settings = Settings {
            ai_sources: vec![
                source("a", ProviderKind::OpenAi, "gpt-4o-mini"),
                source("b", ProviderKind::DeepSeek, "deepseek-chat"),
            ],
            ai_routes: vec![AiRoute::new(AiRouteScope::Agent("meeting".into()), "b")],
            ..Settings::default()
        };
        let (router, factory, _secrets, _publisher, _health, _clock) =
            router(settings, &[("a", "chave"), ("b", "chave")]);
        router
            .complete_for(
                &AiRouteScope::Agent("meeting".into()),
                &CompletionRequest::user("oi"),
            )
            .expect("ok");
        let calls = factory.calls.lock().expect("calls").clone();
        assert_eq!(calls, vec!["deepseek-chat"]);
    }

    #[test]
    fn without_a_route_the_primary_answers() {
        let settings = Settings {
            ai_sources: vec![
                source("a", ProviderKind::OpenAi, "gpt-4o-mini"),
                source("b", ProviderKind::DeepSeek, "deepseek-chat"),
            ],
            ai_routes: vec![AiRoute::new(AiRouteScope::Agent("meeting".into()), "b")],
            ..Settings::default()
        };
        let (router, factory, _secrets, _publisher, _health, _clock) =
            router(settings, &[("a", "chave"), ("b", "chave")]);
        router
            .complete_for(
                &AiRouteScope::Capability(crate::domain::ai_route::AiCapability::Summary),
                &CompletionRequest::user("oi"),
            )
            .expect("ok");
        let calls = factory.calls.lock().expect("calls").clone();
        assert_eq!(calls, vec!["gpt-4o-mini"]);
    }

    #[test]
    fn a_disabled_source_is_never_used() {
        let mut primary = source("a", ProviderKind::OpenAi, "gpt-4o-mini");
        primary.enabled = false;
        let settings = Settings {
            ai_sources: vec![
                primary,
                source("b", ProviderKind::DeepSeek, "deepseek-chat"),
            ],
            ..Settings::default()
        };
        let (router, factory, _secrets, _publisher, _health, _clock) =
            router(settings, &[("a", "chave"), ("b", "chave")]);
        router.complete(&CompletionRequest::user("oi")).expect("ok");
        let calls = factory.calls.lock().expect("calls").clone();
        assert_eq!(calls, vec!["deepseek-chat"]);
    }

    #[test]
    fn no_source_at_all_is_reported() {
        let settings = Settings {
            ai_sources: vec![],
            ..Settings::default()
        };
        let (router, _factory, _secrets, _publisher, _health, _clock) = router(settings, &[]);
        let error = router
            .complete(&CompletionRequest::user("oi"))
            .expect_err("falha");
        assert!(error.to_string().contains("nenhuma fonte de IA habilitada"));
    }

    #[test]
    fn the_default_source_keeps_the_key_in_the_keyring() {
        // Regressão: a chave mora sob `llm.source.{id}` (BR-044).
        let settings = Settings::default();
        let source_id = settings.ai_sources[0].id.clone();
        let (router, _factory, secrets, _publisher, _health, _clock) =
            router(settings, &[(source_id.as_str(), "chave")]);
        router.complete(&CompletionRequest::user("oi")).expect("ok");
        assert!(secrets
            .get(&format!("llm.source.{source_id}"))
            .expect("lê")
            .is_some());
    }
}
