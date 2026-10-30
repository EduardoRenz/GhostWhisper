//! Atualização do resumo em **background**.
//!
//! Um único componente para os dois gatilhos que já existem:
//!
//! - **Perguntar**: quando o resumo está velho, a resposta não espera — o
//!   resumo é atualizado em paralelo para a próxima pergunta (PRD Sessão 15).
//! - **Encerramento**: a sessão que termina sem resumo **ou** sem título ganha
//!   uma geração automática, sem bloquear o `session_stop` (BR-104).
//!
//! O trabalho roda em thread própria com **single-flight** (uma geração por
//! sessão) e **cooldown** depois de uma falha, para não martelar o provedor.
//! Falha nunca sobe para quem chamou: só log.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::ports::clock::Clock;
use crate::ports::event_publisher::{EventPublisher, SessionUpdatedEvent};
use crate::ports::repositories::{SessionRepository, TranscriptionsRepository};
use crate::ports::session_end_observer::SessionEndObserver;
use crate::ports::summary_generator::{SummaryGenerator, SummaryMode, SummaryRequest};

/// Cooldown depois de uma falha do resumo em background (ms).
pub const SUMMARY_REFRESH_COOLDOWN_MS: i64 = 60_000;

/// Reserva de sessões em geração + cooldown de falha.
#[derive(Default)]
struct RefreshState {
    inner: Mutex<RefreshInner>,
}

#[derive(Default)]
struct RefreshInner {
    in_flight: HashSet<String>,
    last_failure_ms: HashMap<String, i64>,
}

impl RefreshState {
    /// Reserva a geração da sessão; `false` = já em voo ou em cooldown.
    fn try_begin(&self, session_id: &str, now_ms: i64) -> bool {
        let Ok(mut inner) = self.inner.lock() else {
            return false;
        };
        if inner.in_flight.contains(session_id) {
            return false;
        }
        if let Some(last) = inner.last_failure_ms.get(session_id) {
            if now_ms.saturating_sub(*last) < SUMMARY_REFRESH_COOLDOWN_MS {
                return false;
            }
        }
        inner.in_flight.insert(session_id.to_string());
        true
    }

    /// Libera a sessão (chamado no fim da thread, inclusive em panic).
    fn finish(&self, session_id: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.in_flight.remove(session_id);
        }
    }

    /// Registra a falha para o cooldown.
    fn note_failure(&self, session_id: &str, now_ms: i64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_failure_ms.insert(session_id.to_string(), now_ms);
        }
    }
}

/// Limpa o "em voo" mesmo se a thread do resumo entrar em panic.
struct RefreshGuard {
    state: Arc<RefreshState>,
    session_id: String,
}

impl Drop for RefreshGuard {
    fn drop(&mut self) {
        self.state.finish(&self.session_id);
    }
}

/// Dispara gerações de resumo em background com single-flight e cooldown.
pub struct SummaryRefresh {
    summaries: Arc<dyn SummaryGenerator>,
    sessions: Arc<dyn SessionRepository>,
    transcriptions: Arc<dyn TranscriptionsRepository>,
    clock: Arc<dyn Clock>,
    publisher: Arc<dyn EventPublisher>,
    state: Arc<RefreshState>,
}

impl SummaryRefresh {
    /// Cria o agendador com as portas resolvidas.
    pub fn new(
        summaries: Arc<dyn SummaryGenerator>,
        sessions: Arc<dyn SessionRepository>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
        clock: Arc<dyn Clock>,
        publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            summaries,
            sessions,
            transcriptions,
            clock,
            publisher,
            state: Arc::new(RefreshState::default()),
        }
    }

    /// Atualiza o resumo da sessão numa thread própria (não bloqueia).
    pub fn spawn(&self, session_id: &str) {
        let now_ms = self.clock.now_ms();
        if !self.state.try_begin(session_id, now_ms) {
            return;
        }

        let generator = Arc::clone(&self.summaries);
        let sessions = Arc::clone(&self.sessions);
        let clock = Arc::clone(&self.clock);
        let publisher = Arc::clone(&self.publisher);
        let state = Arc::clone(&self.state);
        let id = session_id.to_string();
        let spawned = thread::Builder::new()
            .name("summary-refresh".into())
            .spawn(move || {
                let _guard = RefreshGuard {
                    state: Arc::clone(&state),
                    session_id: id.clone(),
                };
                match generator.generate(SummaryRequest {
                    session_id: Some(id.clone()),
                    mode: SummaryMode::Auto,
                }) {
                    Ok(result) => {
                        tracing::debug!(
                            session_id = %id,
                            updated = result.updated,
                            has_more = result.has_more,
                            "resumo atualizado em background"
                        );
                        // O histórico (e o detalhe aberto) recarrega o título e
                        // o resumo sem o usuário pedir (BR-104).
                        let subject = sessions
                            .get(&id)
                            .ok()
                            .flatten()
                            .and_then(|session| session.subject);
                        publisher.emit_session_updated(&SessionUpdatedEvent {
                            session_id: id,
                            subject,
                        });
                    }
                    Err(error) => {
                        tracing::warn!(
                            session_id = %id,
                            "falha ao atualizar o resumo em background: {error}"
                        );
                        state.note_failure(&id, clock.now_ms());
                    }
                }
            });
        if let Err(error) = spawned {
            // A reserva foi feita antes do spawn; libera para a próxima.
            self.state.finish(session_id);
            tracing::warn!("falha ao iniciar thread do resumo em background: {error}");
        }
    }

    /// Gera o resumo **apenas** quando a sessão terminou sem resumo ou sem
    /// título e tem fala final (BR-104).
    pub fn spawn_if_missing(&self, session_id: &str) {
        let session = match self.sessions.get(session_id) {
            Ok(Some(session)) => session,
            Ok(None) => return,
            Err(error) => {
                tracing::warn!(session_id = %session_id, "falha ao ler a sessão encerrada: {error}");
                return;
            }
        };
        if session.overview.is_some() && session.subject.is_some() {
            return;
        }
        match self.transcriptions.latest_final_end_ms(session_id) {
            Ok(Some(_)) => self.spawn(session_id),
            Ok(None) => tracing::debug!(
                session_id = %session_id,
                "sessão encerrada sem fala final; resumo não é necessário"
            ),
            Err(error) => tracing::warn!(
                session_id = %session_id,
                "falha ao verificar a transcrição da sessão encerrada: {error}"
            ),
        }
    }
}

impl SessionEndObserver for SummaryRefresh {
    fn on_session_ended(&self, session_id: &str) {
        self.spawn_if_missing(session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use crate::adapters::persistence::memory::in_memory_session_repository::InMemorySessionRepository;
    use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;
    use crate::adapters::system::system_clock::FixedClock;
    use crate::domain::overview::MeetingSummary;
    use crate::domain::session::{Session, SessionStatus};
    use crate::domain::transcription::{Speaker, TranscriptSegment};
    use crate::error::Result;
    use crate::ports::summary_generator::{SummaryGenerator, SummaryResult};

    #[test]
    fn the_state_is_single_flight_and_cools_down_after_failure() {
        let state = RefreshState::default();

        assert!(state.try_begin("s1", 1_000));
        assert!(!state.try_begin("s1", 1_000), "já está em voo");
        state.finish("s1");
        assert!(state.try_begin("s1", 2_000), "liberou para a próxima");

        state.finish("s1");
        state.note_failure("s1", 3_000);
        assert!(
            !state.try_begin("s1", 3_000 + SUMMARY_REFRESH_COOLDOWN_MS - 1),
            "durante o cooldown não tenta de novo"
        );
        assert!(
            state.try_begin("s1", 3_000 + SUMMARY_REFRESH_COOLDOWN_MS),
            "depois do cooldown volta a tentar"
        );
    }

    /// Sessão encerrada de teste.
    fn ended_session(subject: Option<&str>, overview: Option<&str>) -> Session {
        Session {
            id: "s1".to_string(),
            started_at_ms: 1_000,
            ended_at_ms: Some(60_000),
            provider: "local".to_string(),
            model: None,
            transcription_model: None,
            user_id: None,
            subject: subject.map(str::to_string),
            overview: overview.map(str::to_string),
            status: SessionStatus::Ended,
        }
    }

    fn final_speech() -> TranscriptSegment {
        TranscriptSegment {
            id: "t1".to_string(),
            session_id: "s1".to_string(),
            speaker: Speaker::They,
            text: "fala final".to_string(),
            start_ms: 0,
            end_ms: 500,
            is_final: true,
        }
    }

    /// Resumo fake: conta chamadas, falha se pedido e grava o título na sessão.
    struct FakeGenerator {
        calls: AtomicUsize,
        fail: bool,
        sessions: Arc<InMemorySessionRepository>,
    }

    impl FakeGenerator {
        fn new(sessions: Arc<InMemorySessionRepository>) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                fail: false,
                sessions,
            }
        }

        fn failing(sessions: Arc<InMemorySessionRepository>) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                fail: true,
                sessions,
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl SummaryGenerator for FakeGenerator {
        fn get(&self, _session_id: Option<String>) -> Result<Option<MeetingSummary>> {
            Ok(None)
        }

        fn generate(&self, request: SummaryRequest) -> Result<SummaryResult> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(crate::error::GhostWhisperError::Other(
                    "provider fora".to_string(),
                ));
            }
            let session_id = request.session_id.expect("sessão");
            self.sessions.rename(&session_id, "Título gerado")?;
            let summary = MeetingSummary {
                session_id,
                subject: Some("Título gerado".to_string()),
                markdown: "## Resumo".to_string(),
                summarized_from_ms: 0,
                summarized_until_ms: 500,
                updated_at_ms: 1,
            };
            Ok(SummaryResult {
                summary,
                updated: true,
                has_more: false,
            })
        }
    }

    /// Publicador fake: guarda os eventos de sessão atualizada.
    #[derive(Default)]
    struct RecordingPublisher {
        updates: Mutex<Vec<SessionUpdatedEvent>>,
    }

    impl RecordingPublisher {
        fn updates(&self) -> Vec<SessionUpdatedEvent> {
            self.updates.lock().expect("lock").clone()
        }
    }

    impl EventPublisher for RecordingPublisher {
        fn emit_segment(&self, _segment: &crate::domain::transcription::TranscriptSegment) {}
        fn emit_audio_level(
            &self,
            _channel: crate::ports::session_controller::AudioChannel,
            _level: crate::domain::audio::AudioLevel,
        ) {
        }
        fn emit_session_state(&self, _state: &crate::ports::event_publisher::SessionStateEvent) {}
        fn emit_insight(&self, _suggestion: &crate::ports::suggestion_generator::Suggestion) {}
        fn emit_suggestion(&self, _suggestion: &crate::ports::suggestion_generator::Suggestion) {}
        fn emit_suggestion_cadence(
            &self,
            _cadence: &crate::ports::event_publisher::SuggestionCadenceEvent,
        ) {
        }
        fn emit_asr_status(&self, _status: &crate::domain::transcription::AsrStatus) {}
        fn emit_llm_source_used(&self, _usage: &crate::ports::event_publisher::LlmSourceUsage) {}
        fn emit_journal(&self, _update: &crate::ports::event_publisher::JournalUpdate) {}
        fn emit_session_updated(&self, event: &SessionUpdatedEvent) {
            self.updates.lock().expect("lock").push(event.clone());
        }
    }

    struct Fixture {
        refresh: Arc<SummaryRefresh>,
        generator: Arc<FakeGenerator>,
        sessions: Arc<InMemorySessionRepository>,
        transcriptions: Arc<InMemoryTranscriptionsRepository>,
        publisher: Arc<RecordingPublisher>,
    }

    fn fixture(fail: bool, session: Session) -> Fixture {
        let sessions = Arc::new(InMemorySessionRepository::new());
        sessions.create(&session).expect("sessão");
        let transcriptions = Arc::new(InMemoryTranscriptionsRepository::new());
        let publisher = Arc::new(RecordingPublisher::default());
        let generator = Arc::new(if fail {
            FakeGenerator::failing(Arc::clone(&sessions))
        } else {
            FakeGenerator::new(Arc::clone(&sessions))
        });
        let refresh = Arc::new(SummaryRefresh::new(
            generator.clone(),
            Arc::clone(&sessions) as Arc<dyn SessionRepository>,
            Arc::clone(&transcriptions) as Arc<dyn TranscriptionsRepository>,
            Arc::new(FixedClock(120_000)) as Arc<dyn Clock>,
            Arc::clone(&publisher) as Arc<dyn EventPublisher>,
        ));
        Fixture {
            refresh,
            generator,
            sessions,
            transcriptions,
            publisher,
        }
    }

    /// Espera a condição virar verdadeira (thread em background).
    fn wait_until(mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !condition() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(condition(), "a condição não foi atingida a tempo");
    }

    #[test]
    fn generates_and_announces_the_new_title_when_the_session_ended_without_summary() {
        let f = fixture(false, ended_session(None, None));
        f.transcriptions.insert(&final_speech()).expect("fala");

        f.refresh.spawn_if_missing("s1");

        wait_until(|| !f.publisher.updates().is_empty());
        let updates = f.publisher.updates();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].session_id, "s1");
        assert_eq!(updates[0].subject.as_deref(), Some("Título gerado"));
        assert_eq!(
            f.sessions.get("s1").unwrap().unwrap().subject.as_deref(),
            Some("Título gerado")
        );
    }

    #[test]
    fn generates_when_the_summary_exists_but_the_session_has_no_title() {
        let f = fixture(false, ended_session(None, Some("## Resumo antigo")));
        f.transcriptions.insert(&final_speech()).expect("fala");

        f.refresh.spawn_if_missing("s1");

        wait_until(|| !f.publisher.updates().is_empty());
        assert_eq!(
            f.sessions.get("s1").unwrap().unwrap().subject.as_deref(),
            Some("Título gerado")
        );
    }

    #[test]
    fn skips_when_the_session_already_has_summary_and_title() {
        let f = fixture(false, ended_session(Some("Meu título"), Some("## Resumo")));
        f.transcriptions.insert(&final_speech()).expect("fala");

        f.refresh.spawn_if_missing("s1");
        thread::sleep(Duration::from_millis(100));

        assert!(f.publisher.updates().is_empty());
        assert_eq!(
            f.sessions.get("s1").unwrap().unwrap().subject.as_deref(),
            Some("Meu título"),
            "o título do usuário não muda"
        );
    }

    #[test]
    fn skips_a_session_without_any_final_speech() {
        let f = fixture(false, ended_session(None, None));

        f.refresh.spawn_if_missing("s1");
        thread::sleep(Duration::from_millis(100));

        assert!(f.publisher.updates().is_empty());
    }

    #[test]
    fn a_failure_is_contained_and_cools_down() {
        let f = fixture(true, ended_session(None, None));
        f.transcriptions.insert(&final_speech()).expect("fala");

        f.refresh.spawn_if_missing("s1");
        // A falha não propaga; a próxima tentativa imediata fica no cooldown.
        wait_until(|| {
            f.refresh
                .state
                .inner
                .lock()
                .unwrap()
                .last_failure_ms
                .contains_key("s1")
        });
        f.refresh.spawn_if_missing("s1");
        thread::sleep(Duration::from_millis(100));

        assert_eq!(f.generator.calls(), 1, "cooldown segura a nova tentativa");
        assert!(f.publisher.updates().is_empty());
    }
}
