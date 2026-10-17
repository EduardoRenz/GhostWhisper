//! Publica eventos do domínio para o frontend via Tauri.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::domain::audio::AudioLevel;
use crate::domain::transcription::{AsrStatus, TranscriptSegment};
use crate::ports::event_publisher::{EventPublisher, SessionStateEvent};
use crate::ports::session_controller::AudioChannel;

/// Nome do evento emitido para cada segmento de transcrição.
pub const TRANSCRIPT_SEGMENT_EVENT: &str = "transcript_segment";
/// Nome do evento de nível de áudio.
pub const AUDIO_LEVEL_EVENT: &str = "audio_level";
/// Nome do evento de estado da sessão.
pub const SESSION_STATE_EVENT: &str = "session_state";
/// Nome do evento de insight gerado por atalho.
pub const INSIGHT_EVENT: &str = "insight";
/// Nome do evento de status do motor de transcrição.
pub const ASR_STATUS_EVENT: &str = "asr_status";

/// Payload do evento `audio_level` (canal + nível).
#[derive(Debug, Clone, Serialize)]
pub struct AudioLevelPayload {
    pub channel: &'static str,
    pub rms: f32,
    pub peak: f32,
}

/// Implementa `EventPublisher` emitindo eventos Tauri.
pub struct TauriEventPublisher {
    app: AppHandle,
}

impl TauriEventPublisher {
    /// Cria o publicador a partir do `AppHandle`.
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl EventPublisher for TauriEventPublisher {
    fn emit_segment(&self, segment: &TranscriptSegment) {
        let _ = self.app.emit(TRANSCRIPT_SEGMENT_EVENT, segment);
    }

    fn emit_audio_level(&self, channel: AudioChannel, level: AudioLevel) {
        let _ = self.app.emit(
            AUDIO_LEVEL_EVENT,
            AudioLevelPayload {
                channel: channel.as_str(),
                rms: level.rms,
                peak: level.peak,
            },
        );
    }

    fn emit_session_state(&self, state: &SessionStateEvent) {
        let _ = self.app.emit(SESSION_STATE_EVENT, state);
    }

    fn emit_insight(&self, text: &str) {
        let _ = self.app.emit(INSIGHT_EVENT, text);
    }

    fn emit_asr_status(&self, status: &AsrStatus) {
        let _ = self.app.emit(ASR_STATUS_EVENT, status);
    }
}
