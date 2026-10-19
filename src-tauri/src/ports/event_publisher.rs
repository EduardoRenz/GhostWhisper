//! Porta de publicação de eventos para a UI.

use serde::Serialize;

use crate::domain::audio::AudioLevel;
use crate::domain::transcription::{AsrStatus, TranscriptSegment};
use crate::ports::session_controller::AudioChannel;
use crate::ports::suggestion_generator::Suggestion;

/// Estado da sessão publicado para a UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionStateEvent {
    /// `true` quando há sessão aberta (gravando **ou** pausada).
    pub listening: bool,
    /// `true` quando a captura está suspensa, mas a sessão segue aberta.
    pub paused: bool,
    /// Id da sessão ativa (quando `listening`).
    pub session_id: Option<String>,
    /// Canais ativos: `microphone` e/ou `system`.
    pub channels: Vec<String>,
    /// Início da sessão (relógio de parede, ms) — a UI mostra o tempo decorrido.
    ///
    /// `None` quando não há sessão aberta.
    pub started_at_ms: Option<i64>,
}

/// Estado da cadência do auto-sugerir publicado para a UI (BR-039).
///
/// É o **timer que já existe** no agendador, exposto como feedback visual: a
/// janela de sugestões mostra a contagem regressiva e o anel de progresso. A UI
/// não recalcula nada — o core é a única fonte da verdade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SuggestionCadenceEvent {
    /// O automático está ligado (configuração do usuário).
    pub enabled: bool,
    /// O automático está rodando (ligado e com sessão aberta).
    pub running: bool,
    /// Milissegundos até a próxima janela automática.
    pub remaining_ms: i64,
    /// A janela venceu, mas não há fala nova para sugerir.
    pub awaiting_speech: bool,
    /// Duração total do ciclo (para desenhar o anel de progresso).
    pub window_ms: i64,
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
    /// Emite uma sugestão já persistida (painel ou atalho global).
    fn emit_insight(&self, suggestion: &Suggestion);
    /// Emite a sugestão **efêmera em andamento** (rascunho fast-first + deltas do
    /// streaming). A janela de sugestões substitui o texto a cada evento (BR-028).
    fn emit_suggestion(&self, suggestion: &Suggestion);
    /// Emite a cadência do auto-sugerir (contagem regressiva e anel) — BR-039.
    fn emit_suggestion_cadence(&self, cadence: &SuggestionCadenceEvent);
    /// Emite o status de preparação do motor de transcrição (download/carga).
    fn emit_asr_status(&self, status: &AsrStatus);
}
