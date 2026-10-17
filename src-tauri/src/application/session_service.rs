//! Caso de uso: iniciar/encerrar sessões de transcrição.

use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::domain::session::{Session, SessionStatus};
use crate::domain::transcription::Speaker;
use crate::error::{GhostWhisperError, Result};
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::audio_source::AudioSource;
use crate::ports::clock::Clock;
use crate::ports::event_publisher::{EventPublisher, SessionStateEvent};
use crate::ports::repositories::{SessionRepository, TranscriptionsRepository};
use crate::ports::session_controller::{AudioChannel, SessionController};
use crate::ports::speech_engine_factory::SpeechEngineFactory;

use super::transcription_pipeline::TranscriptionPipeline;

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
}

struct ActiveSession {
    id: String,
    channels: Vec<ActiveChannel>,
}

struct ActiveChannel {
    channel: AudioChannel,
    pipeline: Arc<TranscriptionPipeline>,
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
        }
    }

    fn source(&self, channel: AudioChannel) -> &Arc<dyn AudioSource> {
        match channel {
            AudioChannel::Microphone => &self.microphone,
            AudioChannel::System => &self.system_audio,
        }
    }
}

impl SessionController for SessionService {
    fn start(&self, channels: &[AudioChannel]) -> Result<String> {
        self.stop()?;

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

        let id = Uuid::new_v4().to_string();

        // Persiste a sessão antes de começar a capturar.
        self.sessions.create(&Session {
            id: id.clone(),
            started_at_ms: self.clock.now_ms(),
            ended_at_ms: None,
            provider: "local".to_string(),
            model: None,
            transcription_model: None,
            user_id: None,
            subject: None,
            overview: None,
            status: SessionStatus::Active,
        })?;

        let mut active_channels: Vec<ActiveChannel> = Vec::new();
        let mut last_error: Option<GhostWhisperError> = None;

        for channel in unique {
            let (vad, asr) = match self.engines.create() {
                Ok(engine) => engine,
                Err(error) => {
                    tracing::warn!("falha ao criar motor de fala ({:?}): {error}", channel);
                    last_error = Some(error);
                    continue;
                }
            };

            let speaker = match channel {
                AudioChannel::Microphone => Speaker::Me,
                AudioChannel::System => Speaker::They,
            };

            let pipeline = Arc::new(TranscriptionPipeline::new(
                id.clone(),
                speaker,
                channel,
                vad,
                asr,
                Arc::clone(&self.publisher),
                Arc::clone(&self.transcriptions),
            ))
            .start();

            let consumer: Arc<dyn AudioConsumer> = pipeline.clone();
            match self.source(channel).start(None, consumer) {
                Ok(()) => active_channels.push(ActiveChannel { channel, pipeline }),
                Err(error) => {
                    tracing::warn!("falha ao capturar {:?}: {error}", channel);
                    pipeline.stop();
                    last_error = Some(error);
                }
            }
        }

        if active_channels.is_empty() {
            // Não deixe uma sessão órfã aberta.
            let _ = self.sessions.end(&id, self.clock.now_ms());
            return Err(last_error.unwrap_or_else(|| {
                GhostWhisperError::Other("falha ao iniciar áudio".to_string())
            }));
        }

        let channel_labels: Vec<String> = active_channels
            .iter()
            .map(|active| active.channel.as_str().to_string())
            .collect();

        let mut guard = self
            .active
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de sessão envenenado".to_string()))?;
        *guard = Some(ActiveSession {
            id: id.clone(),
            channels: active_channels,
        });
        drop(guard);

        self.publisher.emit_session_state(&SessionStateEvent {
            listening: true,
            session_id: Some(id.clone()),
            channels: channel_labels,
        });

        Ok(id)
    }

    fn stop(&self) -> Result<()> {
        let active = self
            .active
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de sessão envenenado".to_string()))?
            .take();

        if let Some(active) = active {
            for channel in &active.channels {
                channel.pipeline.stop();
                if let Err(error) = self.source(channel.channel).stop() {
                    tracing::warn!("falha ao parar {:?}: {error}", channel.channel);
                }
            }
            self.sessions.end(&active.id, self.clock.now_ms())?;
            self.publisher.emit_session_state(&SessionStateEvent {
                listening: false,
                session_id: None,
                channels: Vec::new(),
            });
        }
        Ok(())
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
    use crate::domain::transcription::{
        SpeechQuality, SpeechSpan, TranscribedSpan, TranscriptSegment,
    };
    use crate::ports::speech_to_text::SpeechToText;
    use crate::ports::voice_activity::VoiceActivityDetector;

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

    struct NoSpeechVad;
    impl VoiceActivityDetector for NoSpeechVad {
        fn speech_segments(&self, _samples: &[f32]) -> Result<Vec<SpeechSpan>> {
            Ok(vec![])
        }
    }

    struct NullAsr;
    impl SpeechToText for NullAsr {
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
        fn create(&self) -> Result<(Arc<dyn VoiceActivityDetector>, Arc<dyn SpeechToText>)> {
            Ok((Arc::new(NoSpeechVad), Arc::new(NullAsr)))
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
        fn emit_insight(&self, _text: &str) {}
        fn emit_asr_status(&self, _status: &crate::domain::transcription::AsrStatus) {}
    }

    struct Fixture {
        mic: Arc<FakeAudioSource>,
        system: Arc<FakeAudioSource>,
        sessions: Arc<InMemorySessionRepository>,
        publisher: Arc<RecordingPublisher>,
        service: SessionService,
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
}
