//! Configurações não sensíveis em arquivo JSON.

use std::fs;
use std::path::PathBuf;

use crate::domain::settings::Settings;
use crate::error::{GhostWhisperError, Result};
use crate::ports::settings_store::SettingsStore;

/// Persiste `Settings` em `settings.json` (sem segredos).
pub struct FileSettingsStore {
    path: PathBuf,
}

impl FileSettingsStore {
    /// Cria o store apontando para o arquivo `path`.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl SettingsStore for FileSettingsStore {
    fn load(&self) -> Result<Settings> {
        if !self.path.exists() {
            return Ok(Settings::default());
        }
        let raw = fs::read_to_string(&self.path)?;
        if raw.trim().is_empty() {
            return Ok(Settings::default());
        }
        serde_json::from_str(&raw)
            .map(Settings::sanitized)
            .map_err(|e| GhostWhisperError::Config(format!("settings.json inválido: {e}")))
    }

    fn save(&self, settings: &Settings) -> Result<()> {
        settings.validate()?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| GhostWhisperError::Serialization(e.to_string()))?;
        // Escrita atômica: grava em `.tmp` e renomeia.
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileSettingsStore::new(dir.path().join("settings.json"));
        assert_eq!(store.load().expect("load"), Settings::default());
    }

    #[test]
    fn round_trips_settings() {
        use crate::domain::settings::ProviderKind;

        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileSettingsStore::new(dir.path().join("settings.json"));

        let settings = Settings {
            provider: ProviderKind::OpenRouter,
            model: "meta-llama/llama-3.1-8b".into(),
            translation_enabled: true,
            ..Settings::default()
        };

        store.save(&settings).expect("save");
        assert_eq!(store.load().expect("load"), settings);

        // Nada de segredos no arquivo.
        let raw = fs::read_to_string(dir.path().join("settings.json")).expect("read");
        assert!(!raw.to_lowercase().contains("api_key"));
    }

    #[test]
    fn rejects_invalid_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        fs::write(&path, "{ isso não é json").unwrap();
        let store = FileSettingsStore::new(path);
        assert!(store.load().is_err());
    }

    #[test]
    fn save_validates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileSettingsStore::new(dir.path().join("settings.json"));
        let settings = Settings {
            model: "  ".into(),
            ..Settings::default()
        };
        assert!(store.save(&settings).is_err());
    }
}
