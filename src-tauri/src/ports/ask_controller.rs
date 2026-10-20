//! Porta do caso de uso de **pergunta** (aba "Sessão", BR-053).
//!
//! Implementada pela aplicação (`AskService`).

use crate::domain::journal::JournalRequestSource;
use crate::domain::llm::CompletionImage;
use crate::error::Result;
use serde::Serialize;

/// Pedido de pergunta (entrada do caso de uso).
#[derive(Debug, Clone)]
pub struct AskRequest {
    /// Sessão alvo (`None` = a ativa ou a mais recente).
    pub session_id: Option<String>,
    /// A pergunta do usuário.
    pub question: String,
    /// Origem da pergunta (`search`, `preset`, `screenshot`).
    pub source: JournalRequestSource,
    /// Imagens anexadas (prints de tela) — BR-056.
    pub images: Vec<CompletionImage>,
}

/// Resultado de uma pergunta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AskOutcome {
    /// Id da entrada no diário.
    pub entry_id: String,
    /// Resposta final (vazia quando a chamada falhou).
    pub answer: String,
}

/// Pergunta à IA e registra no diário da sessão.
pub trait AskController: Send + Sync {
    /// Faz a pergunta e grava pergunta + resposta no diário.
    fn ask(&self, request: AskRequest) -> Result<AskOutcome>;

    /// Cancela uma pergunta (a entrada fica `cancelled`).
    fn cancel(&self, entry_id: &str) -> Result<()>;
}
