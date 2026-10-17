//! Porta de gerenciamento de sink virtual (fallback de áudio do sistema).

use crate::error::Result;

/// Cria e remove um sink nulo ("virtual") no servidor Pulse/PipeWire.
///
/// Adaptador: `PactlVirtualSinkManager`.
pub trait VirtualSinkManager: Send + Sync {
    /// Cria um sink nulo com o nome dado; retorna o índice do módulo carregado.
    fn create(&self, name: &str) -> Result<u32>;
    /// Remove o módulo (sink) pelo índice retornado na criação.
    fn remove(&self, module_index: u32) -> Result<()>;
}
