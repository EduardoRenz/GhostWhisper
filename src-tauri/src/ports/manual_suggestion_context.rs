//! Porta do contexto manual da Transcrição para a janela de Sugestões.
//!
//! O botão de IA de uma mensagem grava o contexto aqui; a janela de Sugestões
//! busca e o usuário gera a **sugestão manual** (BR-041). O caso de uso recebe o
//! contexto como parâmetro (`SuggestionRequest.manual_context`); este store é
//! apenas o correio entre as duas janelas Tauri.

use serde::{Deserialize, Serialize};

use crate::domain::transcription::Speaker;
use crate::error::Result;

/// Mensagem selecionada na Transcrição como contexto de uma sugestão manual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualSuggestionContext {
    pub session_id: String,
    pub speaker: Speaker,
    /// Texto completo do segmento (clampado no IPC).
    pub text: String,
    /// Tempo desde o início da sessão (ms).
    pub at_ms: i64,
}

/// Store do contexto manual (no máximo um por vez).
///
/// Adaptador: `InMemoryManualSuggestionContextStore`.
pub trait ManualSuggestionContextStore: Send + Sync {
    /// Grava/substitui o contexto atual.
    fn put(&self, context: ManualSuggestionContext) -> Result<()>;
    /// Devolve o contexto se ele pertencer à sessão pedida.
    fn get(&self, session_id: &str) -> Result<Option<ManualSuggestionContext>>;
    /// Limpa o contexto (troca de sessão ou ✕ na janela).
    fn clear(&self, session_id: &str) -> Result<()>;
}
