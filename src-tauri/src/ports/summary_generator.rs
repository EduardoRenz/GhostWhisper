//! Porta do caso de uso de resumo (inbound port).

use serde::Serialize;

use crate::domain::overview::MeetingSummary;
use crate::error::Result;

/// Como o resumo foi pedido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryMode {
    /// Tick automático: incremental e sem chamar o LLM quando não há novidade.
    Auto,
    /// Pedido explícito do usuário: regenera do zero.
    Manual,
}

/// Pedido de resumo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRequest {
    /// Sessão alvo (`None` = ativa ou a mais recente).
    pub session_id: Option<String>,
    pub mode: SummaryMode,
}

/// Resultado do resumo, com os sinais que a UI usa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SummaryResult {
    pub summary: MeetingSummary,
    /// `true` quando o LLM foi chamado e o conteúdo mudou.
    pub updated: bool,
    /// `true` quando ainda há trechos não processados (próximo tick continua).
    pub has_more: bool,
}

/// Gera e persiste o resumo de uma sessão.
///
/// Implementado pela aplicação (`SummaryService`) e chamado pelo IPC.
pub trait SummaryGenerator: Send + Sync {
    /// Resumo já gravado (`None` se ainda não existe).
    fn get(&self, session_id: Option<String>) -> Result<Option<MeetingSummary>>;
    /// Gera/atualiza o resumo conforme o modo.
    fn generate(&self, request: SummaryRequest) -> Result<SummaryResult>;
}
