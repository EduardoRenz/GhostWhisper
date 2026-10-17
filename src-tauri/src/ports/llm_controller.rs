//! Porta de controle de LLM (inbound port / caso de uso).

use crate::domain::llm::{Completion, CompletionRequest};
use crate::error::Result;

/// Valida a configuração atual e executa completudes.
///
/// Implementado pela aplicação (`LlmRouter`) e chamado pelo IPC.
pub trait LlmController: Send + Sync {
    /// Rótulo do provider/modelo ativo (sem segredos).
    fn provider_label(&self) -> Result<String>;
    /// Testa a conexão ("validar chave"); retorna a resposta do provider.
    fn validate(&self) -> Result<String>;
    /// Executa uma completude com a configuração ativa.
    fn complete(&self, request: &CompletionRequest) -> Result<Completion>;
}
