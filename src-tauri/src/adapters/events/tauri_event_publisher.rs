//! Publica eventos do domínio para o frontend via Tauri.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::domain::audio::AudioLevel;
use crate::domain::transcription::{AsrStatus, TranscriptSegment};
use crate::ports::event_publisher::{EventPublisher, SessionStateEvent, SuggestionCadenceEvent};
use crate::ports::session_controller::AudioChannel;
use crate::ports::suggestion_generator::Suggestion;

/// Nome do evento emitido para cada segmento de transcrição.
pub const TRANSCRIPT_SEGMENT_EVENT: &str = "transcript_segment";
/// Nome do evento de nível de áudio.
pub const AUDIO_LEVEL_EVENT: &str = "audio_level";
/// Nome do evento de estado da sessão.
pub const SESSION_STATE_EVENT: &str = "session_state";
/// Nome do evento de sugestão gerada (painel ou atalho).
pub const INSIGHT_EVENT: &str = "insight";
/// Nome do evento de sugestão efêmera em andamento (fast-first/streaming).
pub const SUGGESTION_EVENT: &str = "suggestion";
/// Nome do evento de status do motor de transcricao.
pub const ASR_STATUS_EVENT: &str = "asr_status";
/// Nome do evento de cadencia do auto-sugerir (BR-039).
pub const SUGGESTION_CADENCE_EVENT: &str = "suggestion_cadence";

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

    fn emit_insight(&self, suggestion: &Suggestion) {
        let _ = self.app.emit(INSIGHT_EVENT, suggestion);
    }

    fn emit_suggestion(&self, suggestion: &Suggestion) {
        let _ = self.app.emit(SUGGESTION_EVENT, suggestion);
    }

    fn emit_suggestion_cadence(&self, cadence: &SuggestionCadenceEvent) {
        let _ = self.app.emit(SUGGESTION_CADENCE_EVENT, cadence);
    }

    fn emit_asr_status(&self, status: &AsrStatus) {
        let _ = self.app.emit(ASR_STATUS_EVENT, status);
    }
}
