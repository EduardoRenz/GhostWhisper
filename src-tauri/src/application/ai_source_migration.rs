//! Migração das configurações e chaves **anteriores** às fontes de IA.
//!
//! Quem usava o GhostWhisper antes tinha `provider`/`model`/`base_url` nas
//! settings e **uma** chave por *tipo* de provider no keyring (`llm.openai`).
//! Com as fontes múltiplas (BR-044) a chave passa a ser **por fonte**
//! (`llm.source.{id}`).
//!
//! A migração:
//!
//! 1. transforma a configuração legada em **uma** fonte (id `default`);
//! 2. **copia** a chave legada para a chave da nova fonte (a antiga **não** é
//!    apagada — apagar seria destrutivo).
//!
//! É **idempotente**: rodar de novo não duplica fontes nem chaves.

use std::sync::Arc;

use crate::domain::ai_source::AiSource;
use crate::error::Result;
use crate::ports::secret_store::SecretStore;
use crate::ports::settings_store::SettingsStore;

/// Resultado da migração (para o log do boot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MigrationOutcome {
    /// Uma fonte foi criada a partir da configuração legada.
    pub source_created: bool,
    /// Uma chave legada foi copiada para a chave da fonte.
    pub key_copied: bool,
}

/// Migra as configurações e chaves legadas para o modelo de fontes.
pub struct AiSourceMigration {
    settings: Arc<dyn SettingsStore>,
    secrets: Arc<dyn SecretStore>,
}

impl AiSourceMigration {
    /// Cria a migração com as portas resolvidas.
    pub fn new(settings: Arc<dyn SettingsStore>, secrets: Arc<dyn SecretStore>) -> Self {
        Self { settings, secrets }
    }

    /// Executa a migração (idempotente).
    pub fn migrate(&self) -> Result<MigrationOutcome> {
        let mut outcome = MigrationOutcome::default();
        let settings = self.settings.load()?;

        // 1) Configuração legada ⇒ persiste o **novo** formato.
        //
        // O `SettingsStore` já devolve as settings *sanitizadas*: a lista de
        // fontes foi montada a partir dos campos legados. O que falta é
        // gravar o arquivo no formato novo (os campos legados nunca mais são
        // serializados — `skip_serializing`).
        if settings.legacy_provider.is_some() {
            self.settings.save(&settings)?;
            outcome.source_created = true;
        }

        // 2) Chave legada ⇒ chave da fonte (sem apagar a antiga).
        for source in &settings.ai_sources {
            if self.copy_legacy_key(source)? {
                outcome.key_copied = true;
            }
        }

        Ok(outcome)
    }

    /// Copia a chave legada do tipo para a chave da fonte (se existir).
    fn copy_legacy_key(&self, source: &AiSource) -> Result<bool> {
        let new_key = source.secret_key();
        let existing = self
            .secrets
            .get(&new_key)?
            .filter(|value| !value.trim().is_empty());
        if existing.is_some() {
            return Ok(false);
        }
        let legacy_key = source.kind.legacy_secret_key();
        let legacy = self
            .secrets
            .get(&legacy_key)?
            .filter(|value| !value.trim().is_empty());
        let Some(legacy) = legacy else {
            return Ok(false);
        };
        self.secrets.set(&new_key, &legacy)?;
        Ok(true)
    }
}

/// Constrói a fonte a partir da configuração legada.
#[cfg(test)]
fn legacy_source(settings: &crate::domain::settings::Settings) -> AiSource {
    let kind = settings
        .legacy_provider
        .unwrap_or(crate::domain::settings::ProviderKind::OpenAi);
    let mut source = AiSource::new(crate::domain::settings::DEFAULT_SOURCE_ID, kind);
    if let Some(model) = settings
        .legacy_model
        .as_ref()
        .filter(|model| !model.trim().is_empty())
    {
        source.model = model.clone();
    }
    if let Some(url) = settings
        .legacy_base_url
        .as_ref()
        .filter(|url| !url.trim().is_empty())
    {
        source.base_url = Some(url.clone());
    }
    source
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::secrets::in_memory_secret_store::InMemorySecretStore;
    use crate::adapters::settings::file_settings_store::FileSettingsStore;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::domain::settings::{ProviderKind, Settings};

    fn legacy_settings() -> Settings {
        Settings {
            ai_sources: vec![],
            legacy_provider: Some(ProviderKind::OpenAi),
            legacy_model: Some("gpt-4o-mini".to_string()),
            legacy_base_url: None,
            ..Settings::default()
        }
    }

    fn migration(settings: Settings) -> (AiSourceMigration, Arc<InMemorySecretStore>) {
        let secrets = Arc::new(InMemorySecretStore::new());
        let migration = AiSourceMigration::new(
            // O store real (`FileSettingsStore`) devolve as settings
            // *sanitizadas*; o duplo reproduz o mesmo contrato.
            Arc::new(InMemorySettingsStore::with_settings(settings.sanitized())),
            secrets.clone(),
        );
        (migration, secrets)
    }

    /// Grava um `settings.json` no formato **antigo** (provider/modelo).
    fn legacy_file(dir: &std::path::Path) -> std::path::PathBuf {
        let path = dir.join("settings.json");
        let mut legacy = serde_json::to_value(Settings::default()).expect("json");
        let object = legacy.as_object_mut().expect("objeto");
        // O formato antigo não conhecia a lista de fontes nem as rotas.
        object.remove("ai_sources");
        object.remove("ai_routes");
        object.insert("provider".into(), serde_json::json!("openai"));
        object.insert("model".into(), serde_json::json!("gpt-4o-mini"));
        std::fs::write(&path, serde_json::to_string_pretty(&legacy).expect("json")).expect("grava");
        path
    }

    #[test]
    fn the_legacy_file_becomes_a_source_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = legacy_file(dir.path());
        let secrets = Arc::new(InMemorySecretStore::new());
        secrets.set("llm.openai", "sk-antiga-1234").expect("set");

        let migration = AiSourceMigration::new(
            Arc::new(FileSettingsStore::new(path.clone())),
            secrets.clone(),
        );
        let outcome = migration.migrate().expect("migra");
        assert!(outcome.source_created);
        assert!(outcome.key_copied);

        // O arquivo agora tem a lista de fontes e **não** tem os campos legados.
        let raw = std::fs::read_to_string(&path).expect("lê");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("json");
        let object = value.as_object().expect("objeto");
        assert!(object.contains_key("ai_sources"));
        assert!(!object.contains_key("provider"));
        assert!(!object.contains_key("model"));
        assert!(!object.contains_key("base_url"));

        // A chave da fonte foi criada e a legada continua lá.
        assert_eq!(
            secrets.get("llm.source.default").expect("lê").as_deref(),
            Some("sk-antiga-1234")
        );
        assert_eq!(
            secrets.get("llm.openai").expect("lê").as_deref(),
            Some("sk-antiga-1234")
        );
    }

    #[test]
    fn migrating_the_file_twice_changes_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = legacy_file(dir.path());
        let secrets = Arc::new(InMemorySecretStore::new());
        secrets.set("llm.openai", "sk-antiga-1234").expect("set");
        let migration = AiSourceMigration::new(
            Arc::new(FileSettingsStore::new(path.clone())),
            secrets.clone(),
        );

        migration.migrate().expect("primeira");
        let after_first = std::fs::read_to_string(&path).expect("lê");

        let outcome = migration.migrate().expect("segunda");
        assert!(!outcome.source_created);
        assert!(!outcome.key_copied);
        assert_eq!(after_first, std::fs::read_to_string(&path).expect("lê"));
    }

    #[test]
    fn legacy_settings_become_one_source() {
        let (migration, _secrets) = migration(legacy_settings());
        let outcome = migration.migrate().expect("migra");
        assert!(outcome.source_created);
        assert!(!outcome.key_copied);
    }

    #[test]
    fn the_migrated_source_keeps_the_legacy_model() {
        let legacy = legacy_settings();
        let source = legacy_source(&legacy);
        assert_eq!(source.id, crate::domain::settings::DEFAULT_SOURCE_ID);
        assert_eq!(source.kind, ProviderKind::OpenAi);
        assert_eq!(source.model, "gpt-4o-mini");
    }

    #[test]
    fn the_legacy_key_is_copied_and_kept() {
        let (migration, secrets) = migration(legacy_settings());
        secrets.set("llm.openai", "sk-antiga-1234").expect("set");
        let outcome = migration.migrate().expect("migra");
        assert!(outcome.key_copied);

        // A chave da fonte existe…
        assert_eq!(
            secrets.get("llm.source.default").expect("lê").as_deref(),
            Some("sk-antiga-1234")
        );
        // …e a antiga continua lá.
        assert_eq!(
            secrets.get("llm.openai").expect("lê").as_deref(),
            Some("sk-antiga-1234")
        );
    }

    #[test]
    fn already_migrated_settings_are_untouched() {
        let (migration, secrets) = migration(Settings::default());
        let outcome = migration.migrate().expect("migra");
        assert!(!outcome.source_created);
        assert!(!outcome.key_copied);
        assert!(secrets.get("llm.source.default").expect("lê").is_none());
    }

    #[test]
    fn an_existing_source_key_is_not_overwritten() {
        let (migration, secrets) = migration(Settings::default());
        secrets.set("llm.openai", "sk-antiga").expect("set");
        secrets.set("llm.source.default", "sk-nova").expect("set");
        let outcome = migration.migrate().expect("migra");
        assert!(!outcome.key_copied);
        assert_eq!(
            secrets.get("llm.source.default").expect("lê").as_deref(),
            Some("sk-nova")
        );
    }
}
