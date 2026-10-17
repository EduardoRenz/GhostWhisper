//! Porta de geração de insights/sugestões (inbound port / caso de uso).

use crate::error::Result;

/// Gera uma sugestão contextual a partir da transcrição recente.
///
/// Implementado pela aplicação (`InsightService`) e chamado pelo IPC.
pub trait InsightGenerator: Send + Sync {
    /// Gera um insight para a sessão dada (`None` = sessão mais recente).
    fn generate(&self, session_id: Option<String>) -> Result<String>;
}
