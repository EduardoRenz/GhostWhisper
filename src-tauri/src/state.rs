//! Estado compartilhado da aplicação (`AppState`).
//!
//! Guarda apenas **portas** (traits) já resolvidas no composition root
//! (`lib.rs`). Nenhum adaptador concreto é armazenado aqui.

use std::sync::Arc;

use crate::ports::ai_source_service::AiSourceService;
use crate::ports::ask_controller::AskController;
use crate::ports::audio_monitor::AudioMonitor;
use crate::ports::audio_source::AudioSource;
use crate::ports::click_through::ClickThroughController;
use crate::ports::clock::Clock;
use crate::ports::dwell_popover::DwellPopoverController;
use crate::ports::event_publisher::EventPublisher;
use crate::ports::gpu_probe::GpuProbe;
use crate::ports::journal_repository::JournalRepository;
use crate::ports::llm_controller::LlmController;
use crate::ports::manual_suggestion_context::ManualSuggestionContextStore;
use crate::ports::overlay_window::OverlayWindowController;
use crate::ports::pending_capture::PendingCapture;
use crate::ports::repositories::{MessagesRepository, SessionRepository, TranscriptionsRepository};
use crate::ports::screen_capture::ScreenCapture;
use crate::ports::secret_store::SecretStore;
use crate::ports::session_controller::SessionController;
use crate::ports::settings_store::SettingsStore;
use crate::ports::shortcut_registry::GlobalShortcutRegistry;
use crate::ports::suggestion_generator::SuggestionGenerator;
use crate::ports::summary_generator::SummaryGenerator;
use crate::ports::translator::Translator;
use crate::ports::virtual_sink::VirtualSinkManager;

/// Estado injetado nas janelas e comandos Tauri.
#[derive(Clone)]
pub struct AppState {
    /// Controle da janela overlay.
    pub overlay: Arc<dyn OverlayWindowController>,
    /// Coordenação do click-through.
    pub click_through: Arc<dyn ClickThroughController>,
    /// Captura de microfone.
    pub audio: Arc<dyn AudioSource>,
    /// Captura de áudio do sistema (monitor de saída).
    pub system_audio: Arc<dyn AudioSource>,
    /// Monitor atrasado do microfone ("ouvir minha voz").
    pub monitor: Arc<dyn AudioMonitor>,
    /// Gerenciamento do sink virtual.
    pub virtual_sink: Arc<dyn VirtualSinkManager>,
    /// Controle de sessões de transcrição.
    pub session: Arc<dyn SessionController>,
    /// Repositório de sessões (histórico).
    pub sessions: Arc<dyn SessionRepository>,
    /// Repositório de transcrições (histórico e busca).
    pub transcriptions: Arc<dyn TranscriptionsRepository>,
    /// Repositório de mensagens (sugestões da IA).
    pub messages: Arc<dyn MessagesRepository>,
    /// Repositório do diário de perguntas (aba "Sessão", BR-052).
    pub journal: Arc<dyn JournalRepository>,
    /// Perguntas à IA (aba "Sessão", BR-053).
    pub ask: Arc<dyn AskController>,
    /// Captura de tela (print para a pergunta, BR-055).
    pub screen: Arc<dyn ScreenCapture>,
    /// Print capturado e ainda **não enviado** (anexo da pergunta).
    pub capture: Arc<dyn PendingCapture>,
    /// Relógio do sistema (duração de sessões em andamento no export).
    pub clock: Arc<dyn Clock>,
    /// Configurações do usuário (sem segredos).
    pub settings: Arc<dyn SettingsStore>,
    /// Chaves de API (keyring ou fallback de sessão).
    pub secrets: Arc<dyn SecretStore>,
    /// Detecção de GPU (sugestão de modelo local).
    pub gpu: Arc<dyn GpuProbe>,
    /// Roteador de LLM (validar chave/completudes).
    pub llm: Arc<dyn LlmController>,
    /// Gestão das fontes de IA (cadastrar, reordenar, chaves, saúde).
    pub ai_sources: Arc<dyn AiSourceService>,
    /// Tradução ao vivo.
    pub translator: Arc<dyn Translator>,
    /// Resumo incremental da sessão.
    pub summary: Arc<dyn SummaryGenerator>,
    /// Sugestões contextuais por agente.
    pub suggestions: Arc<dyn SuggestionGenerator>,
    /// Contexto manual da Transcrição → Sugestões (BR-041).
    pub manual_context: Arc<dyn ManualSuggestionContextStore>,
    /// Publicação de eventos para a UI.
    pub publisher: Arc<dyn EventPublisher>,
    /// Registro de atalhos globais.
    pub shortcuts: Arc<dyn GlobalShortcutRegistry>,
    /// Janela de confirmação do drill-hover (BR-036).
    pub dwell: Arc<dyn DwellPopoverController>,
}

impl AppState {
    /// Monta o estado a partir das portas já resolvidas.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        overlay: Arc<dyn OverlayWindowController>,
        click_through: Arc<dyn ClickThroughController>,
        audio: Arc<dyn AudioSource>,
        system_audio: Arc<dyn AudioSource>,
        monitor: Arc<dyn AudioMonitor>,
        virtual_sink: Arc<dyn VirtualSinkManager>,
        session: Arc<dyn SessionController>,
        sessions: Arc<dyn SessionRepository>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
        messages: Arc<dyn MessagesRepository>,
        journal: Arc<dyn JournalRepository>,
        ask: Arc<dyn AskController>,
        screen: Arc<dyn ScreenCapture>,
        capture: Arc<dyn PendingCapture>,
        clock: Arc<dyn Clock>,
        settings: Arc<dyn SettingsStore>,
        secrets: Arc<dyn SecretStore>,
        gpu: Arc<dyn GpuProbe>,
        llm: Arc<dyn LlmController>,
        ai_sources: Arc<dyn AiSourceService>,
        translator: Arc<dyn Translator>,
        summary: Arc<dyn SummaryGenerator>,
        suggestions: Arc<dyn SuggestionGenerator>,
        manual_context: Arc<dyn ManualSuggestionContextStore>,
        publisher: Arc<dyn EventPublisher>,
        shortcuts: Arc<dyn GlobalShortcutRegistry>,
        dwell: Arc<dyn DwellPopoverController>,
    ) -> Self {
        Self {
            overlay,
            click_through,
            audio,
            system_audio,
            monitor,
            virtual_sink,
            session,
            sessions,
            transcriptions,
            messages,
            journal,
            ask,
            screen,
            capture,
            clock,
            settings,
            secrets,
            gpu,
            llm,
            ai_sources,
            translator,
            summary,
            suggestions,
            manual_context,
            publisher,
            shortcuts,
            dwell,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    use crate::domain::audio::AudioDevice;
    use crate::domain::geometry::{Point, Rect};
    use crate::error::Result;
    use crate::ports::audio_consumer::AudioConsumer;
    use crate::ports::audio_monitor::AudioMonitor;
    use crate::ports::overlay_window::Surface;
    use crate::ports::screen_capture::CapturedImage;
    use crate::ports::session_controller::AudioChannel;
    use crate::ports::suggestion_generator::{
        PinSuggestionRequest, Suggestion, SuggestionGenerator, SuggestionPhase, SuggestionRequest,
        SuggestionTrigger,
    };
    use crate::ports::summary_generator::{SummaryGenerator, SummaryRequest, SummaryResult};

    struct FakeOverlayWindow {
        visible: AtomicBool,
    }
    impl FakeOverlayWindow {
        fn new() -> Self {
            Self {
                visible: AtomicBool::new(true),
            }
        }
    }
    impl OverlayWindowController for FakeOverlayWindow {
        fn show(&self) {
            self.visible.store(true, Ordering::Relaxed);
        }
        fn hide(&self) {
            self.visible.store(false, Ordering::Relaxed);
        }
        fn toggle(&self) -> bool {
            let next = !self.visible.load(Ordering::Relaxed);
            self.visible.store(next, Ordering::Relaxed);
            next
        }
        fn is_visible(&self) -> bool {
            self.visible.load(Ordering::Relaxed)
        }
        fn set_click_through(&self, _enabled: bool) {}
        fn set_stealth(&self, _enabled: bool) {}
        fn set_surface(&self, _surface: Surface) {}
        fn set_anchor_bottom(&self, _anchored: bool) {}
        fn anchor_bottom(&self) -> bool {
            true
        }
        fn outer_position(&self) -> Option<Point> {
            Some(Point { x: 0, y: 0 })
        }
        fn outer_size(&self) -> Option<(i32, i32)> {
            Some((100, 100))
        }
        fn scale_factor(&self) -> f64 {
            1.0
        }
    }

    struct FakeClickThrough {
        regions: Mutex<Vec<Rect>>,
    }
    impl Default for FakeClickThrough {
        fn default() -> Self {
            Self {
                regions: Mutex::new(Vec::new()),
            }
        }
    }
    impl ClickThroughController for FakeClickThrough {
        fn set_enabled(&self, _enabled: bool) {}
        fn set_interactive_regions(&self, regions: Vec<Rect>) {
            if let Ok(mut guard) = self.regions.lock() {
                *guard = regions;
            }
        }
    }

    struct FakeAudioSource;
    impl AudioSource for FakeAudioSource {
        fn list_devices(&self) -> Result<Vec<AudioDevice>> {
            Ok(vec![AudioDevice {
                id: "fake".into(),
                name: "Fake".into(),
                is_default: true,
            }])
        }
        fn start(
            &self,
            _device_id: Option<String>,
            _consumer: Arc<dyn AudioConsumer>,
        ) -> Result<()> {
            Ok(())
        }
        fn stop(&self) -> Result<()> {
            Ok(())
        }
        fn is_running(&self) -> bool {
            false
        }
    }

    struct FakeAudioMonitor;
    impl AudioMonitor for FakeAudioMonitor {
        fn start(&self, _delay_ms: u64) -> Result<()> {
            Ok(())
        }
        fn push(&self, _samples: &[f32]) {}
        fn stop(&self) -> Result<()> {
            Ok(())
        }
        fn is_running(&self) -> bool {
            false
        }
    }

    struct FakeVirtualSink;
    impl VirtualSinkManager for FakeVirtualSink {
        fn create(&self, _name: &str) -> Result<u32> {
            Ok(7)
        }
        fn remove(&self, _module_index: u32) -> Result<()> {
            Ok(())
        }
    }

    struct FakeSessionController;
    impl SessionController for FakeSessionController {
        fn start(&self, _channels: &[AudioChannel]) -> Result<String> {
            Ok("s1".into())
        }
        fn stop(&self) -> Result<()> {
            Ok(())
        }
        fn pause(&self) -> Result<()> {
            Ok(())
        }
        fn resume(&self) -> Result<()> {
            Ok(())
        }
        fn set_channels(&self, _channels: &[AudioChannel]) -> Result<()> {
            Ok(())
        }
        fn continue_session(&self, id: &str, _channels: &[AudioChannel]) -> Result<String> {
            Ok(id.to_string())
        }
        fn set_language(&self, _language: &str) -> Result<()> {
            Ok(())
        }
        fn current_session_id(&self) -> Option<String> {
            Some("s1".into())
        }
    }

    struct FakeGpuProbe;
    impl crate::ports::gpu_probe::GpuProbe for FakeGpuProbe {
        fn probe(&self) -> crate::ports::gpu_probe::GpuInfo {
            crate::ports::gpu_probe::GpuInfo::none("fake", false)
        }
    }

    struct FakeLlm;
    impl LlmController for FakeLlm {
        fn provider_label(&self) -> Result<String> {
            Ok("fake:test".into())
        }
        fn validate(&self) -> Result<String> {
            Ok("ok".into())
        }
        fn complete(
            &self,
            _request: &crate::domain::llm::CompletionRequest,
        ) -> Result<crate::domain::llm::Completion> {
            Ok(crate::domain::llm::Completion {
                text: "ok".into(),
                model: "fake".into(),
                provider: None,
                usage: None,
            })
        }
    }

    struct FakeTranslator;
    impl Translator for FakeTranslator {
        fn translate(&self, text: &str) -> Result<String> {
            Ok(format!("[{text}]"))
        }
    }

    struct FakeSummary;
    impl SummaryGenerator for FakeSummary {
        fn get(
            &self,
            _session_id: Option<String>,
        ) -> Result<Option<crate::domain::overview::MeetingSummary>> {
            Ok(None)
        }
        fn generate(&self, _request: SummaryRequest) -> Result<SummaryResult> {
            Ok(SummaryResult {
                summary: crate::domain::overview::MeetingSummary {
                    session_id: "s1".into(),
                    subject: Some("T".into()),
                    markdown: "## A".into(),
                    summarized_from_ms: 0,
                    summarized_until_ms: 0,
                    updated_at_ms: 0,
                },
                updated: true,
                has_more: false,
            })
        }
    }

    struct FakeSuggestions;
    impl SuggestionGenerator for FakeSuggestions {
        fn generate(&self, _request: SuggestionRequest) -> Result<Suggestion> {
            Ok(Suggestion {
                session_id: "s1".into(),
                mode_id: "meeting".into(),
                mode_name: "Reunião".into(),
                mode_icon: "users".into(),
                mode_color: "cyan".into(),
                generation_id: "g1".into(),
                trigger: SuggestionTrigger::Auto,
                source: None,
                phase: SuggestionPhase::Full,
                title: "Sugestão".into(),
                text: "sugestão".into(),
                at_ms: 0,
                context_until_ms: 0,
                message_id: Some("m1".into()),
                skipped: false,
            })
        }
        fn pin(&self, request: PinSuggestionRequest) -> Result<Suggestion> {
            Ok(Suggestion {
                session_id: request.session_id,
                mode_id: request.mode_id,
                mode_name: "Reunião".into(),
                mode_icon: "users".into(),
                mode_color: "cyan".into(),
                generation_id: "g1".into(),
                trigger: SuggestionTrigger::Now,
                source: None,
                phase: SuggestionPhase::Full,
                title: String::new(),
                text: request.text,
                at_ms: request.at_ms,
                context_until_ms: request.at_ms,
                message_id: Some("m1".into()),
                skipped: false,
            })
        }

        fn last_suggestion(&self, _session_id: &str) -> Result<Option<Suggestion>> {
            Ok(None)
        }
    }

    struct FakePublisher;
    impl EventPublisher for FakePublisher {
        fn emit_segment(&self, _segment: &crate::domain::transcription::TranscriptSegment) {}
        fn emit_audio_level(
            &self,
            _channel: AudioChannel,
            _level: crate::domain::audio::AudioLevel,
        ) {
        }
        fn emit_session_state(&self, _state: &crate::ports::event_publisher::SessionStateEvent) {}
        fn emit_insight(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion_cadence(
            &self,
            _cadence: &crate::ports::event_publisher::SuggestionCadenceEvent,
        ) {
        }
        fn emit_asr_status(&self, _status: &crate::domain::transcription::AsrStatus) {}
        fn emit_llm_source_used(&self, _usage: &crate::ports::event_publisher::LlmSourceUsage) {}
        fn emit_journal(&self, _update: &crate::ports::event_publisher::JournalUpdate) {}
    }

    struct FakeShortcutRegistry;
    impl GlobalShortcutRegistry for FakeShortcutRegistry {
        fn apply(
            &self,
            _bindings: &[crate::domain::shortcuts::ShortcutBinding],
        ) -> Vec<crate::domain::shortcuts::ShortcutConflict> {
            Vec::new()
        }
        fn active(&self) -> Vec<crate::domain::shortcuts::ShortcutBinding> {
            Vec::new()
        }
    }

    /// Dobra da porta de captura de tela.
    struct FakeScreenCapture;
    impl ScreenCapture for FakeScreenCapture {
        fn capture(&self) -> crate::error::Result<CapturedImage> {
            Ok(CapturedImage {
                mime: "image/jpeg".into(),
                data: vec![1, 2, 3],
                width: 1280,
                height: 720,
            })
        }
    }

    /// Dobra da porta de perguntas (aba "Sessão").
    struct FakeAsk;
    impl AskController for FakeAsk {
        fn ask(
            &self,
            _request: crate::ports::ask_controller::AskRequest,
        ) -> crate::error::Result<crate::ports::ask_controller::AskOutcome> {
            Ok(crate::ports::ask_controller::AskOutcome {
                entry_id: "e1".into(),
                answer: "ok".into(),
            })
        }

        fn cancel(&self, _entry_id: &str) -> crate::error::Result<()> {
            Ok(())
        }
    }

    struct FakeDwell {
        open: Mutex<Option<u32>>,
    }    impl crate::ports::dwell_popover::DwellPopoverController for FakeDwell {
        fn show(&self, _anchor: Rect, dwell_ms: u32) -> Result<()> {
            *self.open.lock().unwrap() = Some(dwell_ms);
            Ok(())
        }
        fn hide(&self) -> Result<()> {
            *self.open.lock().unwrap() = None;
            Ok(())
        }
        fn context(&self) -> Option<u32> {
            *self.open.lock().unwrap()
        }
    }

    fn build_state() -> AppState {
        use crate::adapters::ai_sources::in_memory_ai_source_service::InMemoryAiSourceService;
        use crate::adapters::manual::in_memory_manual_suggestion_context_store::InMemoryManualSuggestionContextStore;
        use crate::adapters::persistence::memory::in_memory_journal_repository::InMemoryJournalRepository;
        use crate::adapters::persistence::memory::in_memory_messages_repository::InMemoryMessagesRepository;
        use crate::adapters::persistence::memory::in_memory_session_repository::InMemorySessionRepository;
        use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;
        use crate::adapters::secrets::in_memory_secret_store::InMemorySecretStore;
        use crate::adapters::settings::memory::InMemorySettingsStore;

        AppState::new(
            Arc::new(FakeOverlayWindow::new()),
            Arc::new(FakeClickThrough::default()),
            Arc::new(FakeAudioSource),
            Arc::new(FakeAudioSource),
            Arc::new(FakeAudioMonitor),
            Arc::new(FakeVirtualSink),
            Arc::new(FakeSessionController),
            Arc::new(InMemorySessionRepository::new()),
            Arc::new(InMemoryTranscriptionsRepository::new()),
            Arc::new(InMemoryMessagesRepository::new()),
            Arc::new(InMemoryJournalRepository::new()),
            Arc::new(FakeAsk),
            Arc::new(FakeScreenCapture),
            Arc::new(crate::adapters::pending::in_memory_pending_capture::InMemoryPendingCapture::new()),
            Arc::new(crate::adapters::system::system_clock::FixedClock(1_000_000)),
            Arc::new(InMemorySettingsStore::new()),
            Arc::new(InMemorySecretStore::new()),
            Arc::new(FakeGpuProbe),
            Arc::new(FakeLlm),
            Arc::new(InMemoryAiSourceService::new()),
            Arc::new(FakeTranslator),
            Arc::new(FakeSummary),
            Arc::new(FakeSuggestions),
            Arc::new(InMemoryManualSuggestionContextStore::new()),
            Arc::new(FakePublisher),
            Arc::new(FakeShortcutRegistry),
            Arc::new(FakeDwell {
                open: Mutex::new(None),
            }),
        )
    }

    #[test]
    fn shares_ports_across_clones() {
        let state = build_state();
        let clone = state.clone();

        assert!(!clone.overlay.toggle());
        assert!(!state.overlay.is_visible());
        assert_eq!(state.audio.list_devices().expect("devices").len(), 1);
        assert_eq!(state.system_audio.list_devices().expect("devices").len(), 1);
        assert_eq!(
            state.virtual_sink.create("ghostwhisper").expect("create"),
            7
        );
        assert_eq!(state.session.current_session_id().as_deref(), Some("s1"));
        assert!(state.sessions.list_recent(10).expect("sessions").is_empty());
        assert!(state
            .transcriptions
            .search("x", 10)
            .expect("search")
            .is_empty());
        assert_eq!(state.llm.validate().expect("validate"), "ok");
        assert!(!state.secrets.is_persistent());
        assert!(!state.monitor.is_running(), "monitor desligado por padrão");
        assert_eq!(state.translator.translate("x").expect("translate"), "[x]");
        assert_eq!(
            state
                .suggestions
                .generate(SuggestionRequest::persist(None))
                .expect("sugestão")
                .text,
            "sugestão"
        );
        assert!(
            state
                .summary
                .generate(SummaryRequest {
                    session_id: None,
                    mode: crate::ports::summary_generator::SummaryMode::Auto,
                })
                .expect("resumo")
                .updated
        );
        assert!(state
            .messages
            .list_by_session("s1")
            .expect("messages")
            .is_empty());
    }
}
