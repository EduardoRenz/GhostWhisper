//! Porta de geração de insights/sugestões (inbound port / caso de uso).

use serde::Serialize;

use crate::error::Result;

/// Sugestão gerada **e persistida** na linha temporal da sessão.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InsightSuggestion {
    /// Id da mensagem gravada (permite deduplicar na UI).
    pub message_id: String,
    /// Sessão a que a sugestão pertence.
    pub session_id: String,
    /// Texto sugerido.
    pub text: String,
    /// Milissegundos desde o início da **sessão** (mesma escala das falas).
    pub at_ms: i64,
}

/// Gera uma sugestão contextual a partir da transcrição recente.
///
/// Implementado pela aplicação (`InsightService`) e chamado pelo IPC.
pub trait InsightGenerator: Send + Sync {
    /// Gera e persiste uma sugestão para a sessão dada
    /// (`None` = sessão mais recente).
    fn generate(&self, session_id: Option<String>) -> Result<InsightSuggestion>;
}
