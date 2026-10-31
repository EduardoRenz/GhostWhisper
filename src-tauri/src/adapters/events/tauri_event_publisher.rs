//! Publica eventos do domínio para o frontend via Tauri.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::domain::audio::AudioLevel;
use crate::domain::recording::RecordingState;
use crate::domain::transcription::{AsrStatus, TranscriptSegment};
use crate::ports::event_publisher::{
    EventPublisher, JournalUpdate, LlmSourceUsage, RecordingProgressEvent, SessionStateEvent,
    SessionUpdatedEvent, SessionWarningEvent, SuggestionCadenceEvent,
    SuggestionClassificationEvent, SuggestionModelDownloadEvent, TranscriptActivityEvent,
};
use crate::ports::session_controller::AudioChannel;
use crate::ports::suggestion_generator::Suggestion;

/// Nome do evento emitido para cada segmento de transcrição.
pub const TRANSCRIPT_SEGMENT_EVENT: &str = "transcript_segment";
/// Nome do evento de nível de áudio.
pub const AUDIO_LEVEL_EVENT: &str = "audio_level";
/// Nome do evento de estado da sessão.
pub const SESSION_STATE_EVENT: &str = "session_state";
/// Nome do evento de aviso de canal indisponível (ISS-087).
pub const SESSION_WARNING_EVENT: &str = "session_warning";
/// Nome do evento de sugestão gerada (painel ou atalho).
pub const INSIGHT_EVENT: &str = "insight";
/// Nome do evento de sugestão efêmera em andamento (fast-first/streaming).
pub const SUGGESTION_EVENT: &str = "suggestion";
/// Nome do evento de status do motor de transcricao.
pub const ASR_STATUS_EVENT: &str = "asr_status";
/// Nome do evento de cadencia do auto-sugerir (BR-039).
pub const SUGGESTION_CADENCE_EVENT: &str = "suggestion_cadence";
/// Nome do evento da classificação mais recente (BR-095).
pub const SUGGESTION_CLASSIFICATION_EVENT: &str = "suggestion_classification";
/// Nome do evento de progresso do model manager (BR-099).
pub const SUGGESTION_MODEL_DOWNLOAD_EVENT: &str = "suggestion_model_download";
/// Nome do evento de atividade de transcrição por canal.
pub const TRANSCRIPT_ACTIVITY_EVENT: &str = "transcript_activity";
/// Nome do evento de fonte de IA que respondeu (BR-045).
pub const LLM_SOURCE_USED_EVENT: &str = "llm_source_used";
/// Nome do evento de progresso do diário de perguntas (BR-052).
pub const JOURNAL_EVENT: &str = "journal";
/// Nome do evento de estado da gravação da sessão (BR-065).
pub const RECORDING_STATE_EVENT: &str = "recording_state";
/// Nome do evento de progresso da finalização (BR-067).
pub const RECORDING_PROGRESS_EVENT: &str = "recording_progress";
/// Nome do evento de sessão alterada pelo resumo/título em background (BR-104).
pub const SESSION_UPDATED_EVENT: &str = "session_updated";

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

    fn emit_session_warning(&self, warning: &SessionWarningEvent) {
        let _ = self.app.emit(SESSION_WARNING_EVENT, warning);
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

    fn emit_suggestion_classification(&self, event: &SuggestionClassificationEvent) {
        let _ = self.app.emit(SUGGESTION_CLASSIFICATION_EVENT, event);
    }

    fn emit_suggestion_model_download(&self, event: &SuggestionModelDownloadEvent) {
        let _ = self.app.emit(SUGGESTION_MODEL_DOWNLOAD_EVENT, event);
    }

    fn emit_transcript_activity(&self, activity: &TranscriptActivityEvent) {
        let _ = self.app.emit(TRANSCRIPT_ACTIVITY_EVENT, activity);
    }

    fn emit_asr_status(&self, status: &AsrStatus) {
        let _ = self.app.emit(ASR_STATUS_EVENT, status);
    }

    fn emit_llm_source_used(&self, usage: &LlmSourceUsage) {
        let _ = self.app.emit(LLM_SOURCE_USED_EVENT, usage);
    }

    fn emit_journal(&self, update: &JournalUpdate) {
        let _ = self.app.emit(JOURNAL_EVENT, update);
    }

    fn emit_recording_state(&self, state: &RecordingState) {
        let _ = self.app.emit(RECORDING_STATE_EVENT, state);
    }

    fn emit_recording_progress(&self, progress: &RecordingProgressEvent) {
        let _ = self.app.emit(RECORDING_PROGRESS_EVENT, progress);
    }

    fn emit_session_updated(&self, event: &SessionUpdatedEvent) {
        let _ = self.app.emit(SESSION_UPDATED_EVENT, event);
    }
}
