//! Porta de segredos (chaves de API).

use crate::error::Result;

/// Guarda chaves de API fora do arquivo de configuração.
///
/// Adaptadores: `KeyringSecretStore` (Secret Service), `InMemorySecretStore`
/// (fallback de sessão, sem persistir).
pub trait SecretStore: Send + Sync {
    /// Grava (ou substitui) um segredo.
    fn set(&self, key: &str, value: &str) -> Result<()>;
    /// Recupera um segredo (`None` se ausente).
    fn get(&self, key: &str) -> Result<Option<String>>;
    /// Remove um segredo (idempotente).
    fn delete(&self, key: &str) -> Result<()>;
    /// Indica se o armazenamento é persistente (keyring) ou apenas de sessão.
    fn is_persistent(&self) -> bool;
}
