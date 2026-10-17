//! Porta de armazenamento de configurações não sensíveis.

use crate::domain::settings::Settings;
use crate::error::Result;

/// Carrega/salva as configurações do usuário (nunca segredos).
///
/// Adaptadores: `FileSettingsStore`, `InMemorySettingsStore`.
pub trait SettingsStore: Send + Sync {
    /// Carrega as configurações (padrões quando ainda não existem).
    fn load(&self) -> Result<Settings>;
    /// Persiste as configurações.
    fn save(&self, settings: &Settings) -> Result<()>;
}
