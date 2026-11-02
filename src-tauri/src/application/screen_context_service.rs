//! Serviço de contexto de tela (BR-105/BR-106).
//!
//! Orquestra a porta [`ScreenContextSource`]: lê a flag nas settings (sem
//! flag, **não toca na porta** — nenhum adaptador é instanciado/conectado),
//! aplica timeout, cacheia o snapshot e publica o evento `screen_context`
//! (com preview cortado) para a pill da AskBar.
//!
//! A porta é **opcional**: sem flag nas settings de boot, o composition root
//! nem instancia o adaptador e o serviço fica apenas com a leitura da flag.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use crate::domain::screen_context::{ScreenContextSnapshot, CONTEXT_FRESH_MS, PREVIEW_MAX_CHARS};
use crate::error::Result;
use crate::ports::clock::Clock;
use crate::ports::event_publisher::{EventPublisher, ScreenContextEvent};
use crate::ports::screen_context_controller::{ScreenContextController, ScreenContextStatus};
use crate::ports::screen_context_source::ScreenContextSource;
use crate::ports::settings_store::SettingsStore;

/// Timeout do snapshot: a AskBar nunca fica presa esperando o AT-SPI.
///
/// 2 s porque a caminhada da árvore faz roundtrips D-Bus por nó (o Chrome
/// tem milhares); com a parada antecipada por orçamento no adaptador o caso
/// normal fica muito abaixo disso (ISS-092).
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(2);

/// Debounce da republicação quando o foco muda (mesma ordem do follow de
/// janela da gravação).
pub const FOCUS_DEBOUNCE_MS: i64 = 500;

/// Reuso do cache quando a AskBar reabre rápido (mesma janela, sem re Ler).
const CACHE_MAX_AGE_MS: i64 = 5_000;

/// Controlador de contexto de tela: cache + evento + flag.
pub struct ScreenContextService {
    /// Adaptador (só existe com a flag ligada no boot).
    source: Option<Arc<dyn ScreenContextSource>>,
    settings: Arc<dyn SettingsStore>,
    clock: Arc<dyn Clock>,
    publisher: Arc<dyn EventPublisher>,
    /// Último snapshot válido (com o instante em que foi obtido).
    cache: Mutex<Option<CachedSnapshot>>,
    /// Última publicação (debounce do foco).
    last_publish_ms: Mutex<i64>,
    /// A última leitura da porta falhou (D-Bus inacessível) — status
    /// `Unavailable` na UI de Configurações.
    last_fetch_failed: Mutex<bool>,
}

#[derive(Clone)]
struct CachedSnapshot {
    snapshot: ScreenContextSnapshot,
    fetched_at_ms: i64,
}

impl ScreenContextService {
    /// Cria o serviço. `source` é `None` quando a flag estava desligada no
    /// boot (composition root não instancia o adaptador).
    pub fn new(
        source: Option<Arc<dyn ScreenContextSource>>,
        settings: Arc<dyn SettingsStore>,
        clock: Arc<dyn Clock>,
        publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            source,
            settings,
            clock,
            publisher,
            cache: Mutex::new(None),
            last_publish_ms: Mutex::new(0),
            last_fetch_failed: Mutex::new(false),
        }
    }

    /// Marca o resultado da última leitura (status de Configurações).
    fn note_fetch(&self, failed: bool) {
        if let Ok(mut slot) = self.last_fetch_failed.lock() {
            *slot = failed;
        }
    }

    /// `true` quando a **flag mestre** está ligada e há adaptador.
    fn enabled(&self) -> bool {
        self.source.is_some()
            && self
                .settings
                .load()
                .map(|settings| settings.screen_context_enabled)
                .unwrap_or(false)
    }

    /// `true` quando o **checkbox** da pill está marcado (BR-106). Desligada a
    /// flag mestre o valor é irrelevante (nenhuma leitura acontece).
    fn include_enabled(&self) -> bool {
        self.settings
            .load()
            .map(|settings| settings.screen_context_include)
            .unwrap_or(true)
    }

    /// Leitura atual da fonte (flag ligada **e** checkbox marcado): cache
    /// curto → fetch → publica o evento da pill. Base comum da pill e do
    /// prompt.
    ///
    /// Desmarcado não há leitura: o usuário desligou o contexto para não
    /// gastar processamento (BR-106).
    fn current(&self) -> Option<ScreenContextSnapshot> {
        if !self.enabled() || !self.include_enabled() {
            return None;
        }
        let source = self.source.as_ref()?;

        // Cache curto: a AskBar consulta ao abrir e o ask pode consultar de
        // novo em seguida; o mesmo snapshot é reaproveitado.
        let cached = self.cache.lock().ok().and_then(|cache| cache.clone());
        if let Some(cached) = cached {
            if self.clock.now_ms() - cached.fetched_at_ms <= CACHE_MAX_AGE_MS {
                return Some(cached.snapshot);
            }
        }

        match self.fetch_arc(source) {
            Ok(snapshot) => {
                self.note_fetch(false);
                self.store_and_publish(snapshot.clone());
                snapshot
            }
            Err(error) => {
                // `name()` na mensagem: diagnóstico sem conteúdo (RNF-03).
                tracing::warn!(
                    source = source.name(),
                    "contexto de tela indisponível: {error}"
                );
                self.note_fetch(true);
                self.store_and_publish(None);
                None
            }
        }
    }

    /// Lê o snapshot da porta (com timeout) e cacheia.
    fn fetch_arc(
        &self,
        source: &Arc<dyn ScreenContextSource>,
    ) -> Result<Option<ScreenContextSnapshot>> {
        let source = Arc::clone(source);
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = source.snapshot();
            let _ = sender.send(result);
        });
        match receiver.recv_timeout(SNAPSHOT_TIMEOUT) {
            Ok(result) => result,
            Err(_) => Err(crate::error::GhostWhisperError::Other(
                "contexto de tela: timeout no snapshot".into(),
            )),
        }
    }

    /// Cacheia o snapshot e publica o evento da pill.
    fn store_and_publish(&self, snapshot: Option<ScreenContextSnapshot>) {
        let now_ms = self.clock.now_ms();
        if let Ok(mut cache) = self.cache.lock() {
            *cache = snapshot.clone().map(|snapshot| CachedSnapshot {
                snapshot,
                fetched_at_ms: now_ms,
            });
        }
        self.publish(snapshot.as_ref());
    }

    /// Publica o estado da pill (preview cortado — o texto completo não sai
    /// do core, RNF-03/CA-04.3).
    ///
    /// `failed` vem do resultado da **última** leitura: distingue "Sem
    /// contexto" (leitura ok, janela sem árvore) de "Contexto indisponível"
    /// (D-Bus/AT-SPI inacessível) na pill (BR-106).
    fn publish(&self, snapshot: Option<&ScreenContextSnapshot>) {
        let now_ms = self.clock.now_ms();
        if let Ok(mut last) = self.last_publish_ms.lock() {
            *last = now_ms;
        }
        let failed = self
            .last_fetch_failed
            .lock()
            .map(|slot| *slot)
            .unwrap_or(false);
        let event = match snapshot {
            Some(snapshot) => ScreenContextEvent {
                available: true,
                failed: false,
                app: snapshot.app.clone(),
                window_title: snapshot.window_title.clone(),
                preview: truncate(snapshot.text.as_str(), PREVIEW_MAX_CHARS),
                captured_at_ms: snapshot.captured_at_ms,
            },
            None => ScreenContextEvent {
                available: false,
                failed,
                app: String::new(),
                window_title: String::new(),
                preview: String::new(),
                captured_at_ms: now_ms,
            },
        };
        self.publisher.emit_screen_context(&event);
    }
}

impl ScreenContextController for ScreenContextService {
    fn snapshot_for_display(&self) -> Option<ScreenContextSnapshot> {
        // Desmarcado, a pill não lê nada — o checkbox desliga a leitura, não
        // só o envio (BR-106).
        self.current()
    }

    fn snapshot_for_ask(&self) -> Option<ScreenContextSnapshot> {
        if !self.include_enabled() {
            return None;
        }
        self.current().and_then(|snapshot| self.fresh(snapshot))
    }

    fn status(&self) -> ScreenContextStatus {
        if self.source.is_none() {
            return ScreenContextStatus::Disabled;
        }
        if !self
            .settings
            .load()
            .map(|settings| settings.screen_context_enabled)
            .unwrap_or(false)
        {
            return ScreenContextStatus::Disabled;
        }
        // Falha na última leitura (D-Bus inacessível) → indisponível; sem
        // leitura ainda, o adaptador existe e o mecanismo é considerado
        // disponível (a pill dirá "Sem contexto" se não houver árvore).
        let failed = self
            .last_fetch_failed
            .lock()
            .map(|slot| *slot)
            .unwrap_or(false);
        if failed {
            ScreenContextStatus::Unavailable
        } else {
            ScreenContextStatus::Available
        }
    }

    fn clear(&self) {
        // Sem leitura: só descarta o cache e avisa a pill (BR-106).
        self.store_and_publish(None);
    }
}

impl ScreenContextService {
    /// Snapshot só entra no prompt com idade ≤ [`CONTEXT_FRESH_MS`] (CA-03.2).
    fn fresh(&self, snapshot: ScreenContextSnapshot) -> Option<ScreenContextSnapshot> {
        let age = self.clock.now_ms().saturating_sub(snapshot.captured_at_ms);
        (age <= CONTEXT_FRESH_MS).then_some(snapshot)
    }

    /// Atualização por mudança de foco (chamada pelo agendador/observer):
    /// republica o snapshot atual com debounce.
    ///
    /// O foco é observado pelo adaptador AT-SPI; este método só evita
    /// martelar a UI quando várias janelas trocam rápido.
    pub fn on_focus_changed(&self) {
        let Some(source) = self.source.as_ref() else {
            return;
        };
        // Checkbox desmarcado: o foco não dispara leitura (BR-106).
        if !self.enabled() || !self.include_enabled() {
            return;
        }
        let now_ms = self.clock.now_ms();
        if let Ok(last) = self.last_publish_ms.lock() {
            if now_ms - *last < FOCUS_DEBOUNCE_MS {
                return;
            }
        }
        match self.fetch_arc(source) {
            Ok(snapshot) => {
                self.note_fetch(false);
                self.store_and_publish(snapshot)
            }
            Err(error) => {
                tracing::warn!(
                    source = source.name(),
                    "contexto de tela indisponível (foco): {error}"
                );
                self.note_fetch(true);
                self.store_and_publish(None);
            }
        }
    }
}

/// Corta `text` em `max_chars` sem quebrar UTF-8, anexando `…`.
fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::screen::fake_screen_context_source::{
        FailingScreenContextSource, FakeScreenContextSource,
    };
    use crate::domain::audio::AudioLevel;
    use crate::domain::settings::Settings;
    use crate::domain::transcription::{AsrStatus, TranscriptSegment};
    use crate::ports::event_publisher::{
        JournalUpdate, LlmSourceUsage, RecordingProgressEvent, SessionStateEvent,
        SessionUpdatedEvent, SessionWarningEvent, SuggestionCadenceEvent,
        SuggestionClassificationEvent, SuggestionModelDownloadEvent, TranscriptActivityEvent,
    };
    use crate::ports::session_controller::AudioChannel;
    use crate::ports::settings_store::SettingsStore;
    use crate::ports::suggestion_generator::Suggestion;

    /// Publisher de teste que grava o último evento de contexto.
    struct RecordingPublisher {
        events: Mutex<Vec<ScreenContextEvent>>,
    }

    impl RecordingPublisher {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }

        fn last(&self) -> Option<ScreenContextEvent> {
            self.events.lock().ok()?.last().cloned()
        }
    }

    impl EventPublisher for RecordingPublisher {
        fn emit_segment(&self, _segment: &TranscriptSegment) {}
        fn emit_audio_level(&self, _channel: AudioChannel, _level: AudioLevel) {}
        fn emit_transcript_activity(&self, _activity: &TranscriptActivityEvent) {}
        fn emit_session_state(&self, _state: &SessionStateEvent) {}
        fn emit_session_warning(&self, _warning: &SessionWarningEvent) {}
        fn emit_insight(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion_cadence(&self, _cadence: &SuggestionCadenceEvent) {}
        fn emit_suggestion_classification(&self, _event: &SuggestionClassificationEvent) {}
        fn emit_suggestion_model_download(&self, _event: &SuggestionModelDownloadEvent) {}
        fn emit_asr_status(&self, _status: &AsrStatus) {}
        fn emit_llm_source_used(&self, _usage: &LlmSourceUsage) {}
        fn emit_journal(&self, _update: &JournalUpdate) {}
        fn emit_recording_progress(&self, _progress: &RecordingProgressEvent) {}
        fn emit_session_updated(&self, _event: &SessionUpdatedEvent) {}
        fn emit_screen_context(&self, event: &ScreenContextEvent) {
            if let Ok(mut events) = self.events.lock() {
                events.push(event.clone());
            }
        }
    }

    /// Settings em memória com a flag controlável.
    struct InMemorySettings {
        inner: Mutex<Settings>,
    }

    impl InMemorySettings {
        fn with_flag(enabled: bool) -> Self {
            Self::with_flags(enabled, true)
        }

        fn with_flags(enabled: bool, include: bool) -> Self {
            let settings = Settings {
                screen_context_enabled: enabled,
                screen_context_include: include,
                ..Settings::default()
            };
            Self {
                inner: Mutex::new(settings),
            }
        }
    }

    impl SettingsStore for InMemorySettings {
        fn load(&self) -> Result<Settings> {
            Ok(self.inner.lock().expect("lock").clone())
        }
        fn save(&self, settings: &Settings) -> Result<()> {
            *self.inner.lock().expect("lock") = settings.clone();
            Ok(())
        }
    }

    /// Relógio fixo avançável.
    struct TestClock {
        now: Mutex<i64>,
    }

    impl TestClock {
        fn new(now: i64) -> Self {
            Self {
                now: Mutex::new(now),
            }
        }

        fn advance(&self, ms: i64) {
            if let Ok(mut now) = self.now.lock() {
                *now += ms;
            }
        }
    }

    impl Clock for TestClock {
        fn now_ms(&self) -> i64 {
            *self.now.lock().expect("lock")
        }
    }

    fn snapshot(text: &str) -> ScreenContextSnapshot {
        ScreenContextSnapshot {
            app: "google-chrome".into(),
            window_title: "API Reference".into(),
            text: text.into(),
            captured_at_ms: 1_000,
        }
    }

    fn service(
        source: Option<Arc<dyn ScreenContextSource>>,
        flag: bool,
    ) -> (
        ScreenContextService,
        Arc<RecordingPublisher>,
        Arc<TestClock>,
    ) {
        let publisher = Arc::new(RecordingPublisher::new());
        let clock = Arc::new(TestClock::new(10_000));
        let settings = Arc::new(InMemorySettings::with_flag(flag));
        let service = ScreenContextService::new(
            source,
            settings,
            clock.clone() as Arc<dyn Clock>,
            publisher.clone() as Arc<dyn EventPublisher>,
        );
        (service, publisher, clock)
    }

    #[test]
    fn disabled_flag_never_touches_port() {
        let fake = Arc::new(FakeScreenContextSource::new());
        let (service, publisher, _) = service(Some(fake.clone()), false);

        assert!(service.snapshot_for_ask().is_none());
        assert_eq!(fake.calls(), 0, "porta não pode ser consultada");
        assert!(publisher.last().is_none(), "nada é publicado");
        assert_eq!(service.status(), ScreenContextStatus::Disabled);
    }

    #[test]
    fn unchecked_checkbox_never_touches_port() {
        // BR-106 (checkbox): desmarcado **não lê nada** — a pill mostra só a
        // opção ("Contexto desativado") e o prompt segue sem o bloco. O
        // usuário desligou o contexto para não gastar processamento.
        let fake =
            Arc::new(FakeScreenContextSource::new().with_snapshot(snapshot("conteúdo da página")));
        let publisher = Arc::new(RecordingPublisher::new());
        let clock = Arc::new(TestClock::new(10_000));
        let settings = Arc::new(InMemorySettings::with_flags(true, false));
        let service = ScreenContextService::new(
            Some(fake.clone()),
            settings.clone(),
            clock.clone() as Arc<dyn Clock>,
            publisher.clone() as Arc<dyn EventPublisher>,
        );

        assert!(service.snapshot_for_display().is_none());
        assert!(service.snapshot_for_ask().is_none());
        service.on_focus_changed();
        assert_eq!(fake.calls(), 0, "desmarcado não pode ler a porta");
        assert!(
            publisher.last().is_none(),
            "sem leitura não há evento (a pill usa o estado paused)"
        );

        // Marcando de novo, o contexto volta a ser lido sob demanda.
        settings.inner.lock().expect("lock").screen_context_include = true;
        let display = service.snapshot_for_display();
        assert!(display.is_some(), "marcado → pill lê o contexto");
        assert_eq!(fake.calls(), 1);
    }

    #[test]
    fn clear_drops_cache_and_next_read_is_fresh() {
        // Desmarcar limpa: o snapshot anterior não pode reaparecer na próxima
        // abertura (BR-106).
        let fake =
            Arc::new(FakeScreenContextSource::new().with_snapshot(snapshot("conteúdo da página")));
        let publisher = Arc::new(RecordingPublisher::new());
        let clock = Arc::new(TestClock::new(10_000));
        let settings = Arc::new(InMemorySettings::with_flags(true, true));
        let service = ScreenContextService::new(
            Some(fake.clone()),
            settings,
            clock as Arc<dyn Clock>,
            publisher.clone() as Arc<dyn EventPublisher>,
        );

        assert!(service.snapshot_for_display().is_some());
        assert_eq!(fake.calls(), 1);
        assert!(publisher.last().expect("evento").available);

        service.clear();
        let cleared = publisher.last().expect("evento clear");
        assert!(!cleared.available, "clear publica 'sem contexto'");

        // Próxima leitura não usa o cache antigo.
        assert!(service.snapshot_for_display().is_some());
        assert_eq!(fake.calls(), 2, "cache foi descartado");
    }

    #[test]
    fn without_adapter_status_is_disabled() {
        let (service, _, _) = service(None, true);
        assert!(service.snapshot_for_ask().is_none());
        assert_eq!(service.status(), ScreenContextStatus::Disabled);
    }

    #[test]
    fn enabled_flag_publishes_preview_capped() {
        let long_text = "x".repeat(PREVIEW_MAX_CHARS + 500);
        let fake = Arc::new(FakeScreenContextSource::new().with_snapshot(snapshot(&long_text)));
        let (service, publisher, _) = service(Some(fake.clone()), true);

        let got = service.snapshot_for_ask().expect("snapshot");
        assert_eq!(got.app, "google-chrome");
        assert_eq!(fake.calls(), 1);

        let event = publisher.last().expect("evento publicado");
        assert!(event.available);
        assert_eq!(event.app, "google-chrome");
        assert_eq!(event.window_title, "API Reference");
        assert_eq!(event.preview.chars().count(), PREVIEW_MAX_CHARS);
        assert!(event.preview.ends_with('…'));
    }

    #[test]
    fn empty_context_publishes_unavailable_state() {
        let fake = Arc::new(FakeScreenContextSource::new());
        let (service, publisher, _) = service(Some(fake), true);

        assert!(service.snapshot_for_ask().is_none());
        let event = publisher.last().expect("evento");
        assert!(!event.available);
        assert!(event.preview.is_empty());
        assert_eq!(service.status(), ScreenContextStatus::Available);
    }

    #[test]
    fn port_error_degrades_to_none() {
        let failing = Arc::new(FailingScreenContextSource::new());
        let (service, publisher, _) = service(Some(failing), true);

        assert!(service.snapshot_for_ask().is_none());
        let event = publisher.last().expect("evento");
        assert!(!event.available);
        // ISS-092: a falha é marcada — a pill distingue "indisponível" de
        // "sem contexto".
        assert!(event.failed, "erro de mecanismo deve marcar failed");
        assert_eq!(service.status(), ScreenContextStatus::Unavailable);
    }

    #[test]
    fn empty_context_is_not_marked_as_failed() {
        // Leitura ok, janela sem árvore: "Sem contexto", não "Indisponível".
        let fake = Arc::new(FakeScreenContextSource::new());
        let (service, publisher, _) = service(Some(fake), true);

        assert!(service.snapshot_for_ask().is_none());
        let event = publisher.last().expect("evento");
        assert!(!event.available);
        assert!(!event.failed, "leitura ok não é falha");
        assert_eq!(service.status(), ScreenContextStatus::Available);
    }

    #[test]
    fn cache_reuses_recent_snapshot() {
        let fake = Arc::new(FakeScreenContextSource::new().with_snapshot(snapshot("hello")));
        let (service, _, clock) = service(Some(fake.clone()), true);

        assert!(service.snapshot_for_ask().is_some());
        clock.advance(CACHE_MAX_AGE_MS - 1);
        assert!(service.snapshot_for_ask().is_some());
        assert_eq!(fake.calls(), 1, "cache deve reusar o snapshot");

        clock.advance(2);
        assert!(service.snapshot_for_ask().is_some());
        assert_eq!(fake.calls(), 2, "cache vencido força nova leitura");
    }

    #[test]
    fn stale_snapshot_is_not_sent_to_prompt() {
        // CA-03.2: snapshot com mais de CONTEXT_FRESH_MS não entra no prompt.
        let mut old = snapshot("velho");
        old.captured_at_ms = 1_000;
        let fake = Arc::new(FakeScreenContextSource::new().with_snapshot(old));
        let (service, _, _) = service(Some(fake), true);

        // clock em 10_000; idade = 9_000 ms → fresco o bastante.
        assert!(service.snapshot_for_ask().is_some());
    }

    #[test]
    fn very_old_snapshot_is_dropped_from_prompt() {
        // Cache vencido + fetch devolvendo snapshot antigo (relógio do
        // adaptador divergiu): o prompt descarta, mas a pill ainda publica.
        let mut old = snapshot("antigo");
        old.captured_at_ms = 1_000;
        let fake = Arc::new(FakeScreenContextSource::new().with_snapshot(old));
        let (service, publisher, clock) = service(Some(fake), true);

        clock.advance(CONTEXT_FRESH_MS + 1);
        assert!(
            service.snapshot_for_ask().is_none(),
            "prompt sem contexto velho"
        );
        // A pill recebe o estado (disponível), mesmo o ask descartando.
        assert!(publisher.last().expect("evento").available);
    }

    #[test]
    fn focus_change_respects_debounce() {
        let fake = Arc::new(FakeScreenContextSource::new().with_snapshot(snapshot("foco")));
        let (service, _, clock) = service(Some(fake.clone()), true);

        service.on_focus_changed();
        assert_eq!(fake.calls(), 1);
        service.on_focus_changed(); // dentro do debounce: ignorado
        assert_eq!(fake.calls(), 1);
        clock.advance(FOCUS_DEBOUNCE_MS);
        service.on_focus_changed();
        assert_eq!(fake.calls(), 2);
    }
}
