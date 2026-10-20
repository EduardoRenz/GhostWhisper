//! Publicador de eventos que **descarta** tudo.
//!
//! Usado quando o app roda sem interface (testes, CLI) e como padrão do
//! roteador de LLM construído sem telemetria.

use crate::domain::audio::AudioLevel;
use crate::domain::transcription::{AsrStatus, TranscriptSegment};
use crate::ports::event_publisher::{
    EventPublisher, JournalUpdate, LlmSourceUsage, SessionStateEvent, SuggestionCadenceEvent,
};
use crate::ports::session_controller::AudioChannel;
use crate::ports::suggestion_generator::Suggestion;

/// Publicador que não publica nada.
pub struct NullEventPublisher;

impl EventPublisher for NullEventPublisher {
    fn emit_segment(&self, _segment: &TranscriptSegment) {}
    fn emit_audio_level(&self, _channel: AudioChannel, _level: AudioLevel) {}
    fn emit_session_state(&self, _state: &SessionStateEvent) {}
    fn emit_insight(&self, _suggestion: &Suggestion) {}
    fn emit_suggestion(&self, _suggestion: &Suggestion) {}
    fn emit_suggestion_cadence(&self, _cadence: &SuggestionCadenceEvent) {}
    fn emit_asr_status(&self, _status: &AsrStatus) {}
    fn emit_llm_source_used(&self, _usage: &LlmSourceUsage) {}
    fn emit_journal(&self, _update: &JournalUpdate) {}
}
