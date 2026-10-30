//! Porta de publicação de eventos para a UI.

use serde::Serialize;

use crate::domain::audio::AudioLevel;
use crate::domain::recording::RecordingState;
use crate::domain::transcription::{AsrStatus, TranscriptSegment};
use crate::ports::session_controller::AudioChannel;
use crate::ports::suggestion_cue_classifier::ClassificationOutcome;
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
    /// A geração automática está em andamento neste instante.
    ///
    /// Publicado **antes** da chamada ao LLM (que pode demorar segundos): é o
    /// feedback imediato de que a sugestão começou a ser gerada, antes do
    /// primeiro evento `suggestion`.
    pub generating: bool,
}

/// Estado do classificador de sugestões (BR-095).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionClassifierState {
    // TODO(task 10): remover os allows quando o scheduler publicar estes estados.
    #[allow(dead_code)]
    Paused,
    #[allow(dead_code)]
    NoSession,
    #[allow(dead_code)]
    Listening,
    Analyzing,
    Ready,
    Error,
}

/// Classificação mais recente publicada para a UI (BR-095).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SuggestionClassificationEvent {
    pub session_id: String,
    pub state: SuggestionClassifierState,
    pub outcome: Option<ClassificationOutcome>,
    pub error: Option<String>,
    pub generating: bool,
}

/// Fase do download/instalação de um modelo do classificador (BR-099).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionModelDownloadPhase {
    Downloading,
    Verifying,
    Installing,
    Ready,
    Failed,
}

/// Progresso do model manager publicado para a UI.
///
/// O evento é emitido arquivo a arquivo; `bytes_done`/`bytes_total` somam o
/// artefato inteiro (não só o arquivo atual), para uma barra estável.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SuggestionModelDownloadEvent {
    pub model_id: String,
    pub version: String,
    pub phase: SuggestionModelDownloadPhase,
    /// Arquivo relativo atual (`model.onnx`).
    pub file: Option<String>,
    /// Índice 1-based do arquivo atual.
    pub file_index: usize,
    pub file_count: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    /// Erro (só em `failed`).
    pub error: Option<String>,
}

/// Fonte de IA que respondeu a uma chamada (BR-045).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LlmSourceUsage {
    /// Fonte que respondeu (`AiSource.id`).
    pub source_id: String,
    /// Nome de exibição da fonte ("DeepSeek").
    pub label: String,
    /// Provedor da fonte (`openai`, `deepseek`, …).
    pub provider: String,
    /// **Modelo** que respondeu ("deepseek-reasoner") — BR-051.
    pub model: String,
    /// Tarefa que pediu a chamada (`agent:meeting`, `capability:Summary`,
    /// `default`) — BR-051.
    pub scope: String,
    /// A fonte respondeu em **fallback** (não era a primeira da cadeia).
    pub is_fallback: bool,
    /// Latência da chamada (ms).
    pub latency_ms: u64,
}

/// Fase de uma entrada do diário (aba "Sessão", BR-052).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JournalPhase {
    /// Pergunta registrada; a resposta ainda não chegou.
    Pending,
    /// Resposta em andamento (texto acumulado).
    Streaming,
    /// Resposta completa.
    Answered,
    /// Falhou.
    Error,
}

/// Progresso de uma entrada do diário publicado para a UI.
///
/// Espelha o evento de sugestão (BR-028): a UI substitui o texto da entrada
/// a cada evento, sem esperar o fim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JournalUpdate {
    /// Entrada do diário.
    pub entry_id: String,
    /// Fase atual.
    pub phase: JournalPhase,
    /// Texto acumulado da resposta (ou a pergunta, enquanto pendente).
    pub text: String,
    /// Mensagem de erro (quando `phase = error`).
    pub error: Option<String>,
}

/// Atividade de transcrição de um canal (fala em processamento).
///
/// Emitido no início da fala e quando o passe final termina. É o sinal que a
/// aba Sessão usa para mostrar que ainda há transcrição chegando antes de o
/// usuário perguntar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TranscriptActivityEvent {
    /// Sessão dona do canal (eventos de sessão antiga são ignorados pela UI).
    pub session_id: String,
    /// `microphone` | `system`.
    pub channel: &'static str,
    /// `true` enquanto a fala está sendo processada.
    pub processing: bool,
}

/// Sessão mudou depois de um resumo/título gerado em background (BR-104).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionUpdatedEvent {
    pub session_id: String,
    /// Título atual (`None` quando ainda não há).
    pub subject: Option<String>,
}

/// Progresso da finalização da gravação (BR-067).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecordingProgressEvent {
    /// Sessão dona da gravação.
    pub session_id: String,
    /// `finalizing` | `ready` | `failed`.
    pub phase: String,
    /// 0–100 (só faz sentido em `finalizing`).
    pub percent: u8,
    /// Caminho do arquivo (em `ready`).
    pub path: Option<String>,
    /// Tamanho do arquivo (em `ready`).
    pub size_bytes: u64,
    /// Duração do arquivo (em `ready`).
    pub duration_ms: i64,
    /// Motivo (em `failed`).
    pub error: Option<String>,
}

/// Publica eventos do domínio para a interface.
///
/// Adaptador: `TauriEventPublisher`; dobra de teste: `RecordingEventPublisher`.
pub trait EventPublisher: Send + Sync {
    /// Emite um segmento de transcrição.
    fn emit_segment(&self, segment: &TranscriptSegment);
    /// Emite o nível de áudio de um canal.
    fn emit_audio_level(&self, channel: AudioChannel, level: AudioLevel);
    /// Emite o início/fim do processamento de fala de um canal.
    ///
    /// Default no-op: fakes de teste que não observam atividade não precisam
    /// implementar (o adaptador Tauri sobrescreve).
    fn emit_transcript_activity(&self, _activity: &TranscriptActivityEvent) {}
    /// Emite uma mudança de estado da sessão.
    fn emit_session_state(&self, state: &SessionStateEvent);
    /// Emite uma sugestão já persistida (painel ou atalho global).
    fn emit_insight(&self, suggestion: &Suggestion);
    /// Emite a sugestão **efêmera em andamento** (rascunho fast-first + deltas do
    /// streaming). A janela de sugestões substitui o texto a cada evento (BR-028).
    fn emit_suggestion(&self, suggestion: &Suggestion);
    /// Emite a cadência do auto-sugerir (contagem regressiva e anel) — BR-039.
    fn emit_suggestion_cadence(&self, cadence: &SuggestionCadenceEvent);
    /// Emite a classificação mais recente do modo classificador (BR-095).
    ///
    /// Default no-op: fakes que não observam classificação não precisam
    /// implementar.
    fn emit_suggestion_classification(&self, _event: &SuggestionClassificationEvent) {}
    /// Emite o progresso do model manager do classificador (BR-099).
    ///
    /// Default no-op: só o adaptador Tauri precisa publicar.
    fn emit_suggestion_model_download(&self, _event: &SuggestionModelDownloadEvent) {}
    /// Emite o status de preparação do motor de transcrição (download/carga).
    fn emit_asr_status(&self, status: &AsrStatus);
    /// Emite qual fonte de IA respondeu (e se foi fallback) — BR-045.
    fn emit_llm_source_used(&self, usage: &LlmSourceUsage);
    /// Emite o progresso de uma entrada do diário (pergunta/resposta) — BR-052.
    fn emit_journal(&self, update: &JournalUpdate);
    /// Emite o estado da gravação da sessão (BR-065).
    ///
    /// Default no-op: fakes que não observam gravação não precisam implementar.
    fn emit_recording_state(&self, _state: &RecordingState) {}
    /// Emite o progresso da finalização da gravação (BR-067).
    fn emit_recording_progress(&self, _progress: &RecordingProgressEvent) {}
    /// Emite a mudança de uma sessão (título/resumo gerado em background) —
    /// BR-104. Default no-op: fakes que não observam o histórico ignoram.
    fn emit_session_updated(&self, _event: &SessionUpdatedEvent) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_event_serializes_snake_case() {
        let event = SuggestionClassificationEvent {
            session_id: "s1".to_string(),
            state: SuggestionClassifierState::NoSession,
            outcome: None,
            error: Some("sem sessão".to_string()),
            generating: false,
        };
        let json = serde_json::to_value(event).expect("json");
        assert_eq!(json["state"], "no_session");
        assert_eq!(json["outcome"], serde_json::Value::Null);
        assert_eq!(json["generating"], false);
    }
}
