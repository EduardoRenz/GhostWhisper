//! Segredos apenas em memória (fallback quando não há keyring).

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::{GhostWhisperError, Result};
use crate::ports::secret_store::SecretStore;

/// Guarda segredos somente durante a execução — **nunca** persiste em disco.
///
/// Usado quando o Secret Service está indisponível (BR-003: fallback seguro,
/// pedindo a chave por sessão sem gravá-la).
pub struct InMemorySecretStore {
    entries: Mutex<HashMap<String, String>>,
}

impl InMemorySecretStore {
    /// Cria o store vazio.
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, String>>> {
        self.entries
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de segredos envenenado".to_string()))
    }
}

impl Default for InMemorySecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for InMemorySecretStore {
    fn set(&self, key: &str, value: &str) -> Result<()> {
        self.lock()?.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(self.lock()?.get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.lock()?.remove(key);
        Ok(())
    }

    fn is_persistent(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_delete() {
        let store = InMemorySecretStore::new();
        assert!(!store.is_persistent());
        assert_eq!(store.get("k").unwrap(), None);
        store.set("k", "v").unwrap();
        assert_eq!(store.get("k").unwrap().as_deref(), Some("v"));
        store.delete("k").unwrap();
        assert_eq!(store.get("k").unwrap(), None);
        // delete é idempotente
        store.delete("k").unwrap();
    }
}
