//! Segredos no keyring do sistema (Secret Service).

use crate::error::{GhostWhisperError, Result};
use crate::ports::secret_store::SecretStore;

/// Serviço usado no keyring para entradas do GhostWhisper.
const SERVICE: &str = "ghostwhisper";
/// Serviço do nome anterior, consultado apenas como fallback de leitura para
/// não obrigar o usuário a digitar a chave de API de novo após a renomeação.
const SERVICE_LEGACY: &str = "opensua";

/// Guarda chaves de API no keyring (Secret Service no Linux).
pub struct KeyringSecretStore;

impl KeyringSecretStore {
    /// Cria o store (não contata o backend).
    pub fn new() -> Self {
        Self
    }

    /// Verifica se o backend está acessível (sondando uma entrada inexistente).
    ///
    /// `NoEntry` indica backend acessível; qualquer outro erro indica
    /// indisponibilidade (ex.: sem Secret Service).
    pub fn is_available(&self) -> bool {
        match keyring::Entry::new(SERVICE, "__ghostwhisper_probe") {
            Ok(entry) => matches!(entry.get_password(), Ok(_) | Err(keyring::Error::NoEntry)),
            Err(_) => false,
        }
    }

    fn entry(key: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(SERVICE, key)
            .map_err(|e| GhostWhisperError::Other(format!("keyring indisponível: {e}")))
    }
}

impl Default for KeyringSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeyringSecretStore {
    fn set(&self, key: &str, value: &str) -> Result<()> {
        Self::entry(key)?
            .set_password(value)
            .map_err(|e| GhostWhisperError::Other(format!("falha ao gravar no keyring: {e}")))
    }

    fn get(&self, key: &str) -> Result<Option<String>> {
        match Self::entry(key)?.get_password() {
            Ok(secret) => return Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => {}
            Err(e) => {
                return Err(GhostWhisperError::Other(format!(
                    "falha ao ler do keyring: {e}"
                )))
            }
        }
        // Nada no serviço novo: tenta o antigo (renomeação).
        match keyring::Entry::new(SERVICE_LEGACY, key)
            .map_err(|e| GhostWhisperError::Other(format!("keyring indisponível: {e}")))?
            .get_password()
        {
            Ok(secret) => {
                tracing::info!("chave recuperada do serviço antigo do keyring");
                Ok(Some(secret))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(GhostWhisperError::Other(format!(
                "falha ao ler do keyring: {e}"
            ))),
        }
    }

    fn delete(&self, key: &str) -> Result<()> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(GhostWhisperError::Other(format!(
                "falha ao remover do keyring: {e}"
            ))),
        }
    }

    fn is_persistent(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Requer um Secret Service ativo; rode com `--ignored`.
    #[test]
    #[ignore = "requer Secret Service (keyring) no ambiente"]
    fn stores_and_reads_secret_in_keyring() {
        let store = KeyringSecretStore::new();
        assert!(store.is_available(), "keyring indisponível");
        let key = "llm.__test__";
        store.set(key, "segredo-de-teste").expect("set");
        assert_eq!(
            store.get(key).expect("get").as_deref(),
            Some("segredo-de-teste")
        );
        store.delete(key).expect("delete");
        assert_eq!(store.get(key).expect("get pós-delete"), None);
    }
}
