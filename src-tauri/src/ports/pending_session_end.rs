//! Porta do **resumo de fim de sessão** pendente (modal "Sessão Finalizada").
//!
//! A janela `session-end` é aberta pelo overlay e lê o contexto ao montar —
//! mesmo padrão do print pendente (`PendingCapture`).

use crate::error::Result;

/// Guarda o id da última sessão encerrada que o modal deve mostrar.
pub trait PendingSessionEnd: Send + Sync {
    /// Define a sessão do modal.
    fn set(&self, session_id: &str) -> Result<()>;
    /// Lê a sessão sem removê-la (o modal pode recarregar).
    fn get(&self) -> Result<Option<String>>;
    /// Limpa ao fechar o modal.
    fn clear(&self) -> Result<()>;
}
