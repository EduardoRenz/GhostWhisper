//! Caso de uso: iniciar/encerrar/pausar/continuar sessões de transcrição.
//!
//! Conceito de sessão (BR-019): a sessão é a unidade de continuidade. Pausar
//! suspende a captura e **mantém a sessão aberta** (mesmo id); retomar volta a
//! capturar nela; continuar reativa uma sessão encerrada. Só `start` cria um id
//! novo. Os timestamps dos segmentos são relativos ao início da sessão, então a
//! linha temporal segue de onde parou (transposto do Perssua: `pause/resume` da
//! prévia de transcrição + `DatabaseManager.resumeSession`).
//!
//! Os **canais capturados podem ser trocados a qualquer momento**
//! ([`set_channels`](SessionController::set_channels)) sem encerrar a sessão.

use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::domain::session::{Session, SessionStatus};
use crate::domain::transcription::Speaker;
use crate::error::{GhostWhisperError, Result};
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::audio_source::AudioSource;
use crate::ports::clock::Clock;
use crate::ports::event_publisher::{EventPublisher, SessionStateEvent};
use crate::ports::recording::RecordingLifecycle;
use crate::ports::repositories::{SessionRepository, TranscriptionsRepository};
use crate::ports::session_controller::{AudioChannel, SessionController};
use crate::ports::speech_engine_factory::SpeechEngineFactory;

use super::transcription_pipeline::TranscriptionPipeline;
use super::turn_scheduler::TurnTimings;

/// Orquestra a sessão: persiste, escolhe as fontes, monta os pipelines e inicia.
pub struct SessionService {
    microphone: Arc<dyn AudioSource>,
    system_audio: Arc<dyn AudioSource>,
    engines: Arc<dyn SpeechEngineFactory>,
    publisher: Arc<dyn EventPublisher>,
    sessions: Arc<dyn SessionRepository>,
    transcriptions: Arc<dyn TranscriptionsRepository>,
    clock: Arc<dyn Clock>,
    active: Mutex<Option<ActiveSession>>,
    /// Ultima escolha de canais da UI: herdada pela sessao aberta
    /// automaticamente (ex.: pela pergunta) e pela proxima captura.
    last_channels: Mutex<Vec<AudioChannel>>,
    /// Gancho opcional da gravação (BR-065). Ausente em testes que não gravam.
    recording: Option<Arc<dyn RecordingLifecycle>>,
}

/// Canal sendo captado agora: porta de áudio + pipeline de transcrição.
#[derive(Clone)]
struct ActiveChannel {
    channel: AudioChannel,
    pipeline: Arc<TranscriptionPipeline>,
}

/// Sessão aberta (gravando ou pausada).
struct ActiveSession {
    id: String,
    /// Início da sessão (relógio de parede) — preservado ao retomar.
    started_at_ms: i64,
    /// Canais **desejados** (mantidos durante a pausa).
    channels: Vec<AudioChannel>,
    /// Canais captando agora. **Vazio enquanto pausada** (é o que define a pausa).
    live: Vec<ActiveChannel>,
}

impl ActiveSession {
    fn paused(&self) -> bool {
        self.live.is_empty()
    }

    /// Canais exibidos na UI: os que captam agora ou, na pausa, os escolhidos.
    fn labels(&self) -> Vec<String> {
        if self.paused() {
            return self
                .channels
                .iter()
                .map(|channel| channel.as_str().to_string())
                .collect();
        }
        self.live
            .iter()
            .map(|entry| entry.channel.as_str().to_string())
            .collect()
    }
}

impl SessionService {
    /// Cria o serviço com as portas resolvidas.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        microphone: Arc<dyn AudioSource>,
        system_audio: Arc<dyn AudioSource>,
        engines: Arc<dyn SpeechEngineFactory>,
        publisher: Arc<dyn EventPublisher>,
        sessions: Arc<dyn SessionRepository>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            microphone,
            system_audio,
            engines,
            publisher,
            sessions,
            transcriptions,
            clock,
            active: Mutex::new(None),
            last_channels: Mutex::new(Vec::new()),
            recording: None,
        }
    }

    /// Liga o ciclo da gravação ao da sessão (BR-065).
    pub fn with_recording(mut self, recording: Arc<dyn RecordingLifecycle>) -> Self {
        self.recording = Some(recording);
        self
    }

    /// Executa um gancho de gravação; falha **nunca** derruba a sessão (RNF-07).
    fn recording_hook(&self, hook: impl FnOnce(&dyn RecordingLifecycle) -> Result<()>) {
        if let Some(recording) = &self.recording {
            if let Err(error) = hook(recording.as_ref()) {
                tracing::warn!("gancho de gravação falhou: {error}");
            }
        }
    }

    fn source(&self, channel: AudioChannel) -> &Arc<dyn AudioSource> {
        match channel {
            AudioChannel::Microphone => &self.microphone,
            AudioChannel::System => &self.system_audio,
        }
    }

    /// Valida e deduplica os canais pedidos.
    fn normalize_channels(channels: &[AudioChannel]) -> Result<Vec<AudioChannel>> {
        let mut unique: Vec<AudioChannel> = Vec::new();
        for channel in channels {
            if !unique.contains(channel) {
                unique.push(*channel);
            }
        }
        if unique.is_empty() {
            return Err(GhostWhisperError::Config(
                "selecione ao menos um canal de áudio".to_string(),
            ));
        }
        Ok(unique)
    }

    /// Guarda a escolha de canais da UI (vale para a sessao aberta sem
    /// captura e para a proxima).
    fn remember_channels(&self, channels: &[AudioChannel]) {
        if let Ok(mut guard) = self.last_channels.lock() {
            *guard = channels.to_vec();
        }
    }

    /// Ultima escolha conhecida (vazia = nunca escolheu).
    fn remembered_channels(&self) -> Vec<AudioChannel> {
        self.last_channels
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// A sessao aberta esta **pausada** (sem capturar)?
    fn is_paused(&self) -> bool {
        self.active
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(ActiveSession::paused))
            .unwrap_or(false)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Option<ActiveSession>>> {
        self.active
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de sessão envenenado".to_string()))
    }

    fn emit(
        &self,
        listening: bool,
        paused: bool,
        session_id: Option<String>,
        channels: Vec<String>,
        started_at_ms: Option<i64>,
    ) {
        self.publisher.emit_session_state(&SessionStateEvent {
            listening,
            paused,
            session_id,
            channels,
            started_at_ms,
        });
    }

    /// Publica o estado de uma sessão aberta (gravando ou pausada).
    fn emit_active(&self, active: &ActiveSession) {
        self.emit(
            true,
            active.paused(),
            Some(active.id.clone()),
            active.labels(),
            Some(active.started_at_ms),
        );
    }

    /// Publica o estado "sem sessão".
    fn emit_idle(&self) {
        self.emit(false, false, None, Vec::new(), None);
    }

    /// Origem de tempo (ms) desta captura dentro da sessão.
    fn origin_ms(&self, started_at_ms: i64) -> i64 {
        (self.clock.now_ms() - started_at_ms).max(0)
    }

    /// Monta **um** canal (motor de fala + pipeline + captura).
    ///
    /// Devolve `None` quando o canal não pôde ser aberto (já registrado no log);
    /// quem chama decide se isso é fatal (nenhum canal) ou parcial.
    fn open_channel(
        &self,
        session_id: &str,
        origin_ms: i64,
        channel: AudioChannel,
    ) -> Option<ActiveChannel> {
        let asr = match self.engines.create() {
            Ok(engine) => engine,
            Err(error) => {
                tracing::warn!("falha ao criar motor de fala ({channel:?}): {error}");
                return None;
            }
        };

        let speaker = match channel {
            AudioChannel::Microphone => Speaker::Me,
            AudioChannel::System => Speaker::They,
        };

        let pipeline = Arc::new(
            TranscriptionPipeline::new(
                session_id.to_string(),
                speaker,
                channel,
                asr,
                Arc::clone(&self.publisher),
                Arc::clone(&self.transcriptions),
            )
            .with_timings(TurnTimings::from_env())
            .with_time_origin(origin_ms),
        )
        .start();

        let consumer: Arc<dyn AudioConsumer> = pipeline.clone();
        match self.source(channel).start(None, consumer) {
            Ok(()) => Some(ActiveChannel { channel, pipeline }),
            Err(error) => {
                tracing::warn!("falha ao capturar {channel:?}: {error}");
                pipeline.stop();
                None
            }
        }
    }

    /// Persiste uma sessao nova e a deixa **ativa**.
    ///
    /// `capture = false` abre a sessao **pausada** (nenhum canal captando): e a
    /// sessao que a pergunta garante quando o usuario ainda nao iniciou a
    /// transcricao (BR-019/BR-052).
    fn open_session(&self, channels: Vec<AudioChannel>, capture: bool) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let started_at_ms = self.clock.now_ms();

        // Persiste a sessao antes de comecar a capturar.
        self.sessions.create(&Session {
            id: id.clone(),
            started_at_ms,
            ended_at_ms: None,
            provider: "local".to_string(),
            model: None,
            transcription_model: None,
            user_id: None,
            subject: None,
            overview: None,
            status: SessionStatus::Active,
        })?;

        let live: Vec<ActiveChannel> = if capture {
            let origin_ms = self.origin_ms(started_at_ms);
            channels
                .iter()
                .filter_map(|channel| self.open_channel(&id, origin_ms, *channel))
                .collect()
        } else {
            Vec::new()
        };

        if capture && live.is_empty() {
            // Nao deixe uma sessao orfa aberta.
            let _ = self.sessions.end(&id, self.clock.now_ms());
            return Err(GhostWhisperError::Other(
                "falha ao iniciar audio".to_string(),
            ));
        }

        let recording_channels = channels.clone();
        let active = ActiveSession {
            id: id.clone(),
            started_at_ms,
            channels,
            live,
        };
        self.emit_active(&active);
        {
            let mut guard = self.lock()?;
            *guard = Some(active);
        }
        self.recording_hook(|recording| {
            recording.on_session_started(&id, &recording_channels, capture)
        });
        Ok(id)
    }

    /// Para a captura de um canal (pipeline + fonte).
    fn close_channel(&self, entry: &ActiveChannel) {
        entry.pipeline.stop();
        if let Err(error) = self.source(entry.channel).stop() {
            tracing::warn!("falha ao parar {:?}: {error}", entry.channel);
        }
    }

    /// Para a captura da sessão ativa (sem encerrá-la).
    fn stop_capture(&self, active: &ActiveSession) {
        for entry in &active.live {
            self.close_channel(entry);
        }
    }
}

impl SessionController for SessionService {
    fn start(&self, channels: &[AudioChannel]) -> Result<String> {
        let channels = Self::normalize_channels(channels)?;
        self.remember_channels(&channels);

        // Já existe uma sessão (aberta pela pergunta, por exemplo): a captura
        // **continua nela** — o diário e o tempo seguem no mesmo id (BR-019).
        if let Some(id) = self.current_session_id() {
            self.set_channels(&channels)?;
            self.resume()?;
            if self.is_paused() {
                return Err(GhostWhisperError::Other(
                    "falha ao iniciar áudio".to_string(),
                ));
            }
            tracing::info!(session_id = id, channels = ?channels, "captura retomada na sessão aberta");
            return Ok(id);
        }

        tracing::info!(channels = ?channels, "sessão iniciada");
        self.open_session(channels, true)
    }

    fn ensure_session(&self) -> Result<String> {
        if let Some(id) = self.current_session_id() {
            return Ok(id);
        }
        // Sem sessão: abre uma **pausada**. A pergunta passa a ter sessão
        // (histórico, diário e tempo); a captura só começa quando o usuário
        // mandar (BR-019, BR-052).
        let remembered = self.remembered_channels();
        let channels = if remembered.is_empty() {
            vec![AudioChannel::Microphone]
        } else {
            remembered
        };
        let id = self.open_session(channels, false)?;
        tracing::info!(
            session_id = id,
            "sessão aberta pausada (pergunta sem gravação)"
        );
        Ok(id)
    }

    fn stop(&self) -> Result<()> {
        let active = self.lock()?.take();
        if let Some(active) = active {
            self.stop_capture(&active);
            // Na pausa as fontes já pararam; parar de novo é idempotente.
            for channel in &active.channels {
                let _ = self.source(*channel).stop();
            }
            self.sessions.end(&active.id, self.clock.now_ms())?;
            self.emit_idle();
            self.recording_hook(|recording| recording.on_session_stopped());
        }
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        let (id, started_at_ms, channels, live) = {
            let mut guard = self.lock()?;
            let Some(active) = guard.as_mut() else {
                return Ok(());
            };
            if active.paused() {
                return Ok(());
            }
            (
                active.id.clone(),
                active.started_at_ms,
                active.channels.clone(),
                std::mem::take(&mut active.live),
            )
        };

        // Fora do lock: `pipeline.stop()` fecha o turno em aberto e pode aguardar
        // o passe final (até 20 s) — nada de segurar o estado da sessão nisso.
        for entry in &live {
            self.close_channel(entry);
        }

        tracing::info!(session_id = id, "sessão pausada (sessão segue aberta)");
        let labels = channels
            .iter()
            .map(|channel| channel.as_str().to_string())
            .collect();
        self.emit(true, true, Some(id), labels, Some(started_at_ms));
        self.recording_hook(|recording| recording.on_session_paused());
        Ok(())
    }

    fn resume(&self) -> Result<()> {
        let snapshot = {
            let guard = self.lock()?;
            match guard.as_ref() {
                Some(active) if active.paused() => Some((
                    active.id.clone(),
                    active.started_at_ms,
                    active.channels.clone(),
                )),
                _ => None,
            }
        };
        let Some((id, started_at_ms, channels)) = snapshot else {
            return Ok(());
        };

        let origin_ms = self.origin_ms(started_at_ms);
        let live: Vec<ActiveChannel> = channels
            .iter()
            .filter_map(|channel| self.open_channel(&id, origin_ms, *channel))
            .collect();

        let mut missing = false;
        let labels = {
            let mut guard = self.lock()?;
            match guard.as_mut() {
                Some(active) => {
                    active.live = live.clone();
                    active.labels()
                }
                None => {
                    missing = true;
                    Vec::new()
                }
            }
        };
        if missing {
            // A sessão foi encerrada enquanto retomávamos: não deixe o áudio
            // capturando sem sessão (o stop pode demorar; fora do lock).
            for entry in &live {
                self.close_channel(entry);
            }
            return Ok(());
        }
        tracing::info!(session_id = id, "sessão retomada");
        self.recording_hook(|recording| recording.on_session_resumed());
        self.emit(true, false, Some(id), labels, Some(started_at_ms));
        Ok(())
    }

    fn set_channels(&self, channels: &[AudioChannel]) -> Result<()> {
        let channels = Self::normalize_channels(channels)?;
        // A escolha vale mesmo sem sessão (a próxima herda) — BR-019.
        self.remember_channels(&channels);

        let (id, started_at_ms, paused, live, labels) = {
            let mut guard = self.lock()?;
            let Some(active) = guard.as_mut() else {
                // Sem sessão aberta não há captura para mexer: a escolha é da UI.
                return Ok(());
            };
            let previous = std::mem::take(&mut active.live);
            active.channels = channels.clone();
            (
                active.id.clone(),
                active.started_at_ms,
                previous.is_empty(),
                previous,
                active.labels(),
            )
        };

        if paused {
            // Pausada: só guarda a escolha; o retomar captura o novo conjunto.
            tracing::info!(session_id = id, channels = ?channels, "canais da sessão pausada atualizados");
            self.emit(true, true, Some(id), labels, Some(started_at_ms));
            return Ok(());
        }

        let (mut kept, removed): (Vec<ActiveChannel>, Vec<ActiveChannel>) = live
            .into_iter()
            .partition(|entry| channels.contains(&entry.channel));

        // Tira os canais que saíram (fora do lock: o stop pode demorar).
        for entry in &removed {
            self.close_channel(entry);
        }

        // Liga os que entraram, com a **mesma origem de tempo** da sessão.
        let origin_ms = self.origin_ms(started_at_ms);
        for channel in &channels {
            if kept.iter().any(|entry| entry.channel == *channel) {
                continue;
            }
            if let Some(entry) = self.open_channel(&id, origin_ms, *channel) {
                kept.push(entry);
            }
        }

        if kept.is_empty() {
            // Nenhum canal sobreviveu: não deixa a sessão "gravando" sem áudio.
            let _ = self.sessions.end(&id, self.clock.now_ms());
            {
                let mut guard = self.lock()?;
                *guard = None;
            }
            self.emit_idle();
            return Err(GhostWhisperError::Other(
                "nenhum canal de áudio pôde ser capturado".to_string(),
            ));
        }

        // Preserva a ordem pedida pelo usuário.
        kept.sort_by_key(|entry| {
            channels
                .iter()
                .position(|channel| *channel == entry.channel)
                .unwrap_or(usize::MAX)
        });

        let labels = {
            let mut guard = self.lock()?;
            match guard.as_mut() {
                Some(active) => {
                    active.live = kept;
                    active.labels()
                }
                None => labels,
            }
        };
        tracing::info!(session_id = id, channels = ?labels, "canais da sessão trocados");
        self.emit(true, false, Some(id), labels, Some(started_at_ms));
        Ok(())
    }

    fn continue_session(&self, id: &str, channels: &[AudioChannel]) -> Result<String> {
        // Encerra a sessão ativa atual (se houver) antes de reabrir a pedida.
        self.stop()?;

        let session = self
            .sessions
            .get(id)?
            .ok_or_else(|| GhostWhisperError::Config(format!("sessão {id} não encontrada")))?;
        let channels = Self::normalize_channels(channels)?;
        self.remember_channels(&channels);

        if session.status == SessionStatus::Ended {
            self.sessions.resume(id)?;
        }

        let origin_ms = self.origin_ms(session.started_at_ms);
        let live: Vec<ActiveChannel> = channels
            .iter()
            .filter_map(|channel| self.open_channel(id, origin_ms, *channel))
            .collect();

        if live.is_empty() {
            // Reabrimos algo que não grava: devolve ao estado encerrado.
            let _ = self.sessions.end(id, self.clock.now_ms());
            return Err(GhostWhisperError::Other(
                "falha ao iniciar áudio".to_string(),
            ));
        }

        let recording_channels = channels.clone();
        let active = ActiveSession {
            id: id.to_string(),
            started_at_ms: session.started_at_ms,
            channels,
            live,
        };
        self.emit_active(&active);
        {
            let mut guard = self.lock()?;
            *guard = Some(active);
        }
        // Continuar uma sessão antiga é uma nova captura: a gravação ganha um
        // arquivo novo (o anterior não é concatenado nem sobrescrito).
        self.recording_hook(|recording| {
            recording.on_session_started(id, &recording_channels, true)
        });
        Ok(id.to_string())
    }

    fn set_language(&self, language: &str) -> Result<()> {
        // O idioma é parâmetro de cada passe: aplica ao motor em cache, sem
        // recarregar o modelo. Sem sessão ativa, é no-op (a escolha persiste nas
        // settings e vale na próxima sessão — BR-034).
        self.engines.update_language(language)
    }

    fn current_session_id(&self) -> Option<String> {
        self.active
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|active| active.id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::adapters::persistence::memory::in_memory_session_repository::InMemorySessionRepository;
    use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;
    use crate::adapters::system::system_clock::FixedClock;
    use crate::domain::audio::AudioDevice;
    use crate::domain::transcription::{SpeechQuality, TranscribedSpan, TranscriptSegment};
    use crate::ports::speech_to_text::SpeechToText;

    struct FakeAudioSource {
        started: AtomicBool,
        fail: bool,
    }
    impl FakeAudioSource {
        fn new(fail: bool) -> Self {
            Self {
                started: AtomicBool::new(false),
                fail,
            }
        }
    }
    impl AudioSource for FakeAudioSource {
        fn list_devices(&self) -> Result<Vec<AudioDevice>> {
            Ok(vec![])
        }
        fn start(
            &self,
            _device_id: Option<String>,
            _consumer: Arc<dyn AudioConsumer>,
        ) -> Result<()> {
            if self.fail {
                return Err(GhostWhisperError::Other("fonte indisponível".to_string()));
            }
            self.started.store(true, Ordering::Relaxed);
            Ok(())
        }
        fn stop(&self) -> Result<()> {
            self.started.store(false, Ordering::Relaxed);
            Ok(())
        }
        fn is_running(&self) -> bool {
            self.started.load(Ordering::Relaxed)
        }
    }

    struct NoSpeechAsr;
    impl SpeechToText for NoSpeechAsr {
        fn transcribe(
            &self,
            _samples: &[f32],
            _quality: SpeechQuality,
        ) -> Result<Vec<TranscribedSpan>> {
            Ok(vec![])
        }
    }

    struct FakeEngineFactory;
    impl SpeechEngineFactory for FakeEngineFactory {
        fn create(&self) -> Result<Arc<dyn SpeechToText>> {
            Ok(Arc::new(NoSpeechAsr))
        }
    }

    struct RecordingPublisher {
        states: Mutex<Vec<SessionStateEvent>>,
    }
    impl EventPublisher for RecordingPublisher {
        fn emit_segment(&self, _segment: &TranscriptSegment) {}
        fn emit_audio_level(
            &self,
            _channel: AudioChannel,
            _level: crate::domain::audio::AudioLevel,
        ) {
        }
        fn emit_session_state(&self, state: &SessionStateEvent) {
            self.states.lock().unwrap().push(state.clone());
        }
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
    }

    struct Fixture {
        mic: Arc<FakeAudioSource>,
        system: Arc<FakeAudioSource>,
        sessions: Arc<InMemorySessionRepository>,
        publisher: Arc<RecordingPublisher>,
        service: SessionService,
    }

    impl Fixture {
        /// Último estado publicado.
        fn last_state(&self) -> SessionStateEvent {
            self.publisher
                .states
                .lock()
                .unwrap()
                .last()
                .cloned()
                .expect("estado")
        }

        fn states(&self) -> Vec<SessionStateEvent> {
            self.publisher.states.lock().unwrap().clone()
        }
    }

    fn fixture(mic_fail: bool, system_fail: bool) -> Fixture {
        let mic = Arc::new(FakeAudioSource::new(mic_fail));
        let system = Arc::new(FakeAudioSource::new(system_fail));
        let sessions = Arc::new(InMemorySessionRepository::new());
        let publisher = Arc::new(RecordingPublisher {
            states: Mutex::new(Vec::new()),
        });
        let service = SessionService::new(
            mic.clone(),
            system.clone(),
            Arc::new(FakeEngineFactory),
            publisher.clone(),
            sessions.clone(),
            Arc::new(InMemoryTranscriptionsRepository::new()),
            Arc::new(FixedClock(1_000)),
        );
        Fixture {
            mic,
            system,
            sessions,
            publisher,
            service,
        }
    }

    #[test]
    fn starts_both_channels_and_persists() {
        let f = fixture(false, false);
        let id = f
            .service
            .start(&[AudioChannel::Microphone, AudioChannel::System])
            .expect("start");
        assert!(f.mic.is_running());
        assert!(f.system.is_running());

        let stored = f.sessions.get(&id).expect("get").expect("present");
        assert_eq!(stored.status, SessionStatus::Active);

        {
            let states = f.publisher.states.lock().unwrap();
            assert_eq!(states.len(), 1);
            assert!(states[0].listening);
            assert!(!states[0].paused);
            assert_eq!(states[0].channels, vec!["microphone", "system"]);
        }

        f.service.stop().expect("stop");
        assert!(!f.mic.is_running());
        assert!(!f.system.is_running());

        let stored = f.sessions.get(&id).expect("get").expect("present");
        assert_eq!(stored.status, SessionStatus::Ended);
        assert_eq!(stored.ended_at_ms, Some(1_000));
    }

    #[test]
    fn continues_with_one_channel_when_other_fails() {
        let f = fixture(false, true);
        let id = f
            .service
            .start(&[AudioChannel::Microphone, AudioChannel::System])
            .expect("start");
        assert!(f.mic.is_running());
        assert!(!f.system.is_running());

        let channels = {
            let states = f.publisher.states.lock().unwrap();
            states[0].channels.clone()
        };
        assert_eq!(channels, vec!["microphone"]);
        assert!(!id.is_empty());
    }

    #[test]
    fn errors_when_all_channels_fail_and_ends_session() {
        let f = fixture(true, true);
        assert!(f
            .service
            .start(&[AudioChannel::Microphone, AudioChannel::System])
            .is_err());
        assert!(f
            .sessions
            .list_recent(10)
            .expect("list")
            .iter()
            .all(|s| { s.status == SessionStatus::Ended }));
    }

    #[test]
    fn rejects_empty_channel_list() {
        let f = fixture(false, false);
        assert!(f.service.start(&[]).is_err());
    }

    #[test]
    fn switching_channels_stops_previous() {
        let f = fixture(false, false);
        f.service.start(&[AudioChannel::Microphone]).expect("mic");
        f.service.start(&[AudioChannel::System]).expect("system");
        assert!(!f.mic.is_running());
        assert!(f.system.is_running());
    }

    #[test]
    fn state_event_carries_the_session_start_for_the_timer() {
        // A UI mostra o tempo decorrido a partir de `started_at_ms` (T04).
        let f = fixture(false, false);
        f.service.start(&[AudioChannel::Microphone]).expect("start");
        assert_eq!(
            f.last_state().started_at_ms,
            Some(1_000),
            "início da sessão (relógio fixo)"
        );

        // Pausar e retomar não zeram o relógio da sessão.
        f.service.pause().expect("pause");
        assert_eq!(f.last_state().started_at_ms, Some(1_000));
        f.service.resume().expect("resume");
        assert_eq!(f.last_state().started_at_ms, Some(1_000));

        f.service.stop().expect("stop");
        let state = f.last_state();
        assert!(!state.listening);
        assert_eq!(state.session_id, None);
        assert_eq!(state.started_at_ms, None, "sem sessão não há tempo");
    }

    // --- Sessão contínua (BR-019) -------------------------------------------

    #[test]
    fn pause_keeps_the_session_open_and_stops_capture() {
        let f = fixture(false, false);
        let id = f.service.start(&[AudioChannel::Microphone]).expect("start");

        f.service.pause().expect("pause");

        assert!(!f.mic.is_running(), "captura suspensa");
        assert_eq!(f.service.current_session_id().as_deref(), Some(id.as_str()));
        let stored = f.sessions.get(&id).expect("get").expect("present");
        assert_eq!(stored.status, SessionStatus::Active, "pausa não encerra");

        let state = f.last_state();
        assert!(state.listening);
        assert!(state.paused);
        assert_eq!(state.session_id.as_deref(), Some(id.as_str()));
    }

    #[test]
    fn pause_is_idempotent_and_without_session_is_a_noop() {
        let f = fixture(false, false);
        f.service.pause().expect("sem sessão");
        assert!(f.states().is_empty(), "não emite estado sem sessão");

        f.service.start(&[AudioChannel::Microphone]).expect("start");
        f.service.pause().expect("pause");
        let emitted = f.states().len();
        f.service.pause().expect("pause de novo");
        assert_eq!(f.states().len(), emitted, "pausar duas vezes não republica");
    }

    #[test]
    fn resume_reopens_the_same_session_and_keeps_started_at() {
        let f = fixture(false, false);
        let id = f
            .service
            .start(&[AudioChannel::Microphone, AudioChannel::System])
            .expect("start");
        f.service.pause().expect("pause");

        f.service.resume().expect("resume");

        assert!(f.mic.is_running());
        assert!(f.system.is_running());
        assert_eq!(f.service.current_session_id().as_deref(), Some(id.as_str()));
        assert_eq!(
            f.sessions.get(&id).expect("get").expect("present").status,
            SessionStatus::Active
        );

        let state = f.last_state();
        assert!(state.listening);
        assert!(!state.paused);
        assert_eq!(state.channels, vec!["microphone", "system"]);
    }

    #[test]
    fn resume_without_pause_is_a_noop() {
        let f = fixture(false, false);
        f.service.resume().expect("sem sessão");
        assert!(f.states().is_empty());

        f.service.start(&[AudioChannel::Microphone]).expect("start");
        let emitted = f.states().len();
        f.service.resume().expect("já gravando");
        assert_eq!(f.states().len(), emitted);
    }

    #[test]
    fn stop_works_while_paused() {
        let f = fixture(false, false);
        let id = f.service.start(&[AudioChannel::Microphone]).expect("start");
        f.service.pause().expect("pause");

        f.service.stop().expect("stop");

        assert_eq!(f.service.current_session_id(), None);
        assert_eq!(
            f.sessions.get(&id).expect("get").expect("present").status,
            SessionStatus::Ended
        );
        let state = f.last_state();
        assert!(!state.listening);
        assert!(!state.paused);
        assert_eq!(state.session_id, None);
    }

    #[test]
    fn starting_after_a_pause_resumes_the_same_session() {
        // A sessão é a unidade de continuidade: começar a capturar de novo
        // **não** abre outro id (BR-019).
        let f = fixture(false, false);
        let first = f.service.start(&[AudioChannel::Microphone]).expect("start");
        f.service.pause().expect("pause");

        let second = f
            .service
            .start(&[AudioChannel::Microphone])
            .expect("retoma a captura");

        assert_eq!(first, second, "mesma sessão");
        assert!(f.mic.is_running());
        assert_eq!(
            f.sessions
                .get(&first)
                .expect("get")
                .expect("present")
                .status,
            SessionStatus::Active
        );
    }

    #[test]
    fn ensure_session_opens_a_paused_session_without_capture() {
        // Perguntar sem transcrição: a sessão existe (histórico/diário), mas
        // nada é captado até o usuário mandar (BR-019, BR-052).
        let f = fixture(false, false);
        let id = f.service.ensure_session().expect("sessão");

        assert!(!f.mic.is_running(), "pausada não captura");
        assert!(!f.system.is_running(), "pausada não captura");

        let stored = f.sessions.get(&id).expect("get").expect("present");
        assert_eq!(stored.status, SessionStatus::Active);

        let state = f.last_state();
        assert!(state.listening, "há sessão aberta");
        assert!(state.paused, "a captura está pausada");
        assert_eq!(state.session_id.as_deref(), Some(id.as_str()));
        assert_eq!(state.started_at_ms, Some(1_000), "o tempo já conta");
        assert_eq!(state.channels, vec!["microphone"], "herda o padrão");
    }

    #[test]
    fn ensure_session_reuses_the_open_session() {
        let f = fixture(false, false);
        let id = f.service.start(&[AudioChannel::Microphone]).expect("start");

        assert_eq!(f.service.ensure_session().expect("sessão"), id);
        assert_eq!(f.states().len(), 1, "não republica estado à toa");
    }

    #[test]
    fn ensure_session_inherits_the_last_chosen_channels() {
        let f = fixture(false, false);
        // Sem sessão o seletor já guarda a escolha para a próxima.
        f.service
            .set_channels(&[AudioChannel::System])
            .expect("guarda a escolha");

        let id = f.service.ensure_session().expect("sessão");
        assert!(!f.system.is_running(), "pausada não captura");
        assert_eq!(f.last_state().channels, vec!["system"]);

        // Começar a capturar usa a escolha e continua na mesma sessão.
        assert_eq!(f.service.start(&[AudioChannel::System]).expect("start"), id);
        assert!(f.system.is_running());
    }

    #[test]
    fn continue_reactivates_an_ended_session_and_keeps_its_start() {
        let f = fixture(false, false);
        let id = f.service.start(&[AudioChannel::Microphone]).expect("start");
        f.service.stop().expect("stop");

        let continued = f
            .service
            .continue_session(&id, &[AudioChannel::Microphone])
            .expect("continue");

        assert_eq!(continued, id, "continua na mesma sessão");
        assert!(f.mic.is_running());
        let stored = f.sessions.get(&id).expect("get").expect("present");
        assert_eq!(stored.status, SessionStatus::Active);
        assert_eq!(stored.ended_at_ms, None, "ended_at é limpo");
        assert_eq!(stored.started_at_ms, 1_000, "início preservado");
    }

    #[test]
    fn continue_ends_the_currently_active_session() {
        let f = fixture(false, false);
        let old = f.service.start(&[AudioChannel::System]).expect("start");
        f.service.stop().expect("stop");

        let current = f
            .service
            .start(&[AudioChannel::Microphone])
            .expect("start atual");
        let continued = f
            .service
            .continue_session(&old, &[AudioChannel::System])
            .expect("continue");

        assert_eq!(continued, old);
        assert_eq!(
            f.sessions
                .get(&current)
                .expect("get")
                .expect("present")
                .status,
            SessionStatus::Ended,
            "a sessão ativa anterior é encerrada"
        );
        assert_eq!(
            f.service.current_session_id().as_deref(),
            Some(old.as_str())
        );
    }

    #[test]
    fn continue_unknown_session_fails() {
        let f = fixture(false, false);
        let error = f
            .service
            .continue_session("nao-existe", &[AudioChannel::Microphone])
            .unwrap_err()
            .to_string();
        assert!(error.contains("não encontrada"), "{error}");
    }

    #[test]
    fn continue_with_all_channels_failing_keeps_the_session_ended() {
        let f = fixture(false, false);
        let id = f.service.start(&[AudioChannel::Microphone]).expect("start");
        f.service.stop().expect("stop");

        let broken = fixture(true, true);
        broken
            .sessions
            .create(&Session {
                id: id.clone(),
                started_at_ms: 1_000,
                ended_at_ms: Some(2_000),
                provider: "local".to_string(),
                model: None,
                transcription_model: None,
                user_id: None,
                subject: None,
                overview: None,
                status: SessionStatus::Ended,
            })
            .expect("create");

        assert!(broken
            .service
            .continue_session(&id, &[AudioChannel::Microphone])
            .is_err());
        assert_eq!(
            broken
                .sessions
                .get(&id)
                .expect("get")
                .expect("present")
                .status,
            SessionStatus::Ended,
            "não deixa a sessão reaberta sem captura"
        );
    }

    // --- Troca de canais durante a sessão ------------------------------------

    #[test]
    fn set_channels_adds_a_channel_without_ending_the_session() {
        let f = fixture(false, false);
        let id = f.service.start(&[AudioChannel::Microphone]).expect("start");

        f.service
            .set_channels(&[AudioChannel::Microphone, AudioChannel::System])
            .expect("set channels");

        assert!(f.mic.is_running(), "o canal antigo continua");
        assert!(f.system.is_running(), "o novo canal começou");
        assert_eq!(
            f.service.current_session_id().as_deref(),
            Some(id.as_str()),
            "mesma sessão"
        );
        let stored = f.sessions.get(&id).expect("get").expect("present");
        assert_eq!(stored.status, SessionStatus::Active);
        assert_eq!(stored.ended_at_ms, None);

        let state = f.last_state();
        assert!(state.listening);
        assert!(!state.paused);
        assert_eq!(state.channels, vec!["microphone", "system"]);
    }

    #[test]
    fn set_channels_removes_a_channel_and_stops_only_it() {
        let f = fixture(false, false);
        f.service
            .start(&[AudioChannel::Microphone, AudioChannel::System])
            .expect("start");

        f.service
            .set_channels(&[AudioChannel::System])
            .expect("set channels");

        assert!(!f.mic.is_running(), "o canal removido parou");
        assert!(f.system.is_running(), "o mantido segue captando");
        assert_eq!(f.last_state().channels, vec!["system"]);
    }

    #[test]
    fn set_channels_swaps_one_for_the_other() {
        let f = fixture(false, false);
        f.service.start(&[AudioChannel::Microphone]).expect("start");

        f.service
            .set_channels(&[AudioChannel::System])
            .expect("set channels");

        assert!(!f.mic.is_running());
        assert!(f.system.is_running());
        assert_eq!(f.last_state().channels, vec!["system"]);
    }

    #[test]
    fn set_channels_while_paused_only_updates_the_choice() {
        let f = fixture(false, false);
        f.service.start(&[AudioChannel::Microphone]).expect("start");
        f.service.pause().expect("pause");

        f.service
            .set_channels(&[AudioChannel::System])
            .expect("set channels");

        assert!(!f.mic.is_running());
        assert!(!f.system.is_running(), "pausado não captura");
        let state = f.last_state();
        assert!(state.paused);
        assert_eq!(state.channels, vec!["system"], "escolha exibida na pausa");

        // O retomar usa o novo conjunto.
        f.service.resume().expect("resume");
        assert!(f.system.is_running());
        assert!(!f.mic.is_running());
        assert_eq!(f.last_state().channels, vec!["system"]);
    }

    #[test]
    fn set_channels_without_a_session_is_a_noop() {
        let f = fixture(false, false);
        f.service
            .set_channels(&[AudioChannel::System])
            .expect("sem sessão");
        assert!(f.states().is_empty(), "não emite estado sem sessão");
    }

    #[test]
    fn set_channels_rejects_an_empty_list() {
        let f = fixture(false, false);
        f.service.start(&[AudioChannel::Microphone]).expect("start");
        assert!(f.service.set_channels(&[]).is_err());
        assert!(f.mic.is_running(), "a captura atual não é afetada");
    }

    #[test]
    fn set_channels_keeps_the_working_one_when_the_other_fails() {
        let f = fixture(false, true);
        f.service.start(&[AudioChannel::Microphone]).expect("start");

        f.service
            .set_channels(&[AudioChannel::Microphone, AudioChannel::System])
            .expect("set channels");

        assert!(f.mic.is_running());
        assert!(!f.system.is_running());
        assert_eq!(f.last_state().channels, vec!["microphone"]);
    }

    #[test]
    fn set_channels_ends_the_session_when_no_channel_survives() {
        // Microfone ok, sistema indisponível: trocar para o sistema remove o
        // único canal vivo e não consegue abrir o novo.
        let f = fixture(false, true);
        let id = f.service.start(&[AudioChannel::Microphone]).expect("start");

        let error = f
            .service
            .set_channels(&[AudioChannel::System])
            .expect_err("sem canal");

        assert!(error.to_string().contains("nenhum canal"), "{error}");
        assert_eq!(f.service.current_session_id(), None);
        assert_eq!(
            f.sessions.get(&id).expect("get").expect("present").status,
            SessionStatus::Ended,
            "não fica gravando sem áudio"
        );
        let state = f.last_state();
        assert!(!state.listening);
        assert_eq!(state.session_id, None);
    }
}
