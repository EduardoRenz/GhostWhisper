//! Porta de registro de atalhos globais.

use crate::domain::shortcuts::{ShortcutBinding, ShortcutConflict};

/// Registra/atualiza atalhos globais do sistema.
///
/// Adaptador: `TauriGlobalShortcutRegistry`.
pub trait GlobalShortcutRegistry: Send + Sync {
    /// Reaplica todos os vínculos; devolve os conflitos (não registrados).
    fn apply(&self, bindings: &[ShortcutBinding]) -> Vec<ShortcutConflict>;
    /// Vínculos registrados no momento.
    fn active(&self) -> Vec<ShortcutBinding>;
}
