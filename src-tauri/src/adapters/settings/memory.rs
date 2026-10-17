//! Dobra de teste: configurações em memória.

use std::sync::Mutex;

use crate::domain::settings::Settings;
use crate::error::{GhostWhisperError, Result};
use crate::ports::settings_store::SettingsStore;

/// `SettingsStore` em memória (testes).
pub struct InMemorySettingsStore {
    inner: Mutex<Settings>,
}

impl InMemorySettingsStore {
    /// Começa com os padrões.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Settings::default()),
        }
    }

    /// Começa com as configurações dadas.
    pub fn with_settings(settings: Settings) -> Self {
        Self {
            inner: Mutex::new(settings),
        }
    }
}

impl Default for InMemorySettingsStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsStore for InMemorySettingsStore {
    fn load(&self) -> Result<Settings> {
        self.inner
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| GhostWhisperError::Other("lock de settings envenenado".to_string()))
    }

    fn save(&self, settings: &Settings) -> Result<()> {
        settings.validate()?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de settings envenenado".to_string()))?;
        *guard = settings.clone();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads() {
        let store = InMemorySettingsStore::new();
        let settings = Settings {
            target_language: "en".into(),
            ..Settings::default()
        };
        store.save(&settings).expect("save");
        assert_eq!(store.load().expect("load").target_language, "en");
    }
}
