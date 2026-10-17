//! Porta de publicação de eventos para a UI.

use serde::Serialize;

use crate::domain::audio::AudioLevel;
use crate::domain::transcription::{AsrStatus, TranscriptSegment};
use crate::ports::session_controller::AudioChannel;

/// Estado da sessão publicado para a UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionStateEvent {
    /// `true` quando há sessão escutando.
    pub listening: bool,
    /// Id da sessão ativa (quando `listening`).
    pub session_id: Option<String>,
    /// Canais ativos: `microphone` e/ou `system`.
    pub channels: Vec<String>,
}

/// Publica eventos do domínio para a interface.
///
/// Adaptador: `TauriEventPublisher`; dobra de teste: `RecordingEventPublisher`.
pub trait EventPublisher: Send + Sync {
    /// Emite um segmento de transcrição.
    fn emit_segment(&self, segment: &TranscriptSegment);
    /// Emite o nível de áudio de um canal.
    fn emit_audio_level(&self, channel: AudioChannel, level: AudioLevel);
    /// Emite uma mudança de estado da sessão.
    fn emit_session_state(&self, state: &SessionStateEvent);
    /// Emite um insight/sugestão gerado fora do painel (ex.: atalho).
    fn emit_insight(&self, text: &str);
    /// Emite o status de preparação do motor de transcrição (download/carga).
    fn emit_asr_status(&self, status: &AsrStatus);
}
