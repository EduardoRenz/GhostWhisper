//! Migração **única** do agente padrão (BR-059).
//!
//! O agente **Padrão** (contextual: identifica a cena e age como o
//! especialista correspondente) entrou no lugar da "Reunião" como agente
//! padrão. Quem já tinha a "Reunião" selecionada — o valor padrão antigo —
//! passa pelo novo agente, que **também** cobre reuniões.
//!
//! A migração é **idempotente e única**: ela grava a marca
//! (`assistant_mode_migrated`) junto com a nova seleção. Depois disso, a
//! escolha do usuário é respeitada (mesmo se ele voltar para a "Reunião").

use std::sync::Arc;

use crate::domain::assistant::{DEFAULT_MODE_ID, LEGACY_DEFAULT_MODE_ID};
use crate::error::Result;
use crate::ports::settings_store::SettingsStore;

/// Resultado da migração (para o log do boot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MigrationOutcome {
    /// O agente selecionado mudou (padrão antigo → agente Padrão).
    pub mode_changed: bool,
    /// A migração rodou (a marca foi gravada).
    pub marked: bool,
}

/// Migra a seleção do agente para o agente **Padrão** (uma vez só).
pub struct AssistantModeMigration {
    settings: Arc<dyn SettingsStore>,
}

impl AssistantModeMigration {
    /// Cria a migração com a porta resolvida.
    pub fn new(settings: Arc<dyn SettingsStore>) -> Self {
        Self { settings }
    }

    /// Executa a migração (idempotente).
    pub fn migrate(&self) -> Result<MigrationOutcome> {
        let mut settings = self.settings.load()?;
        if settings.assistant_mode_migrated {
            return Ok(MigrationOutcome::default());
        }

        settings.assistant_mode_migrated = true;
        let mut outcome = MigrationOutcome {
            mode_changed: false,
            marked: true,
        };
        if settings.assistant_mode_id == LEGACY_DEFAULT_MODE_ID {
            settings.assistant_mode_id = DEFAULT_MODE_ID.to_string();
            outcome.mode_changed = true;
        }
        self.settings.save(&settings)?;
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::settings::file_settings_store::FileSettingsStore;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::domain::settings::Settings;

    #[test]
    fn the_legacy_default_agent_becomes_the_default_agent() {
        let settings = Settings {
            assistant_mode_id: LEGACY_DEFAULT_MODE_ID.to_string(),
            ..Settings::default()
        };
        let store = Arc::new(InMemorySettingsStore::with_settings(settings));
        let migration = AssistantModeMigration::new(store.clone());

        let outcome = migration.migrate().expect("migra");
        assert!(outcome.mode_changed);
        assert!(outcome.marked);
        assert_eq!(store.load().expect("lê").assistant_mode_id, DEFAULT_MODE_ID);
    }

    #[test]
    fn migrating_twice_changes_nothing() {
        let settings = Settings {
            assistant_mode_id: LEGACY_DEFAULT_MODE_ID.to_string(),
            ..Settings::default()
        };
        let store = Arc::new(InMemorySettingsStore::with_settings(settings));
        let migration = AssistantModeMigration::new(store.clone());

        migration.migrate().expect("primeira");
        let after_first = store.load().expect("lê");

        let outcome = migration.migrate().expect("segunda");
        assert!(!outcome.mode_changed);
        assert!(!outcome.marked);
        assert_eq!(after_first, store.load().expect("lê"));
    }

    #[test]
    fn the_choice_of_the_user_is_respected_after_the_migration() {
        // Depois da migração, escolher a "Reunião" de novo **fica**. Com a
        // semeadura (BR-075) a "Reunião" é um agente do usuário.
        let settings = Settings {
            assistant_mode_id: LEGACY_DEFAULT_MODE_ID.to_string(),
            custom_assistant_modes: crate::application::prompts::assistant::factory_modes(),
            assistant_modes_seeded: true,
            ..Settings::default()
        };
        let store = Arc::new(InMemorySettingsStore::with_settings(settings));
        let migration = AssistantModeMigration::new(store.clone());

        migration.migrate().expect("primeira");

        let mut chosen = store.load().expect("lê");
        chosen.assistant_mode_id = LEGACY_DEFAULT_MODE_ID.to_string();
        store.save(&chosen).expect("salva");

        let outcome = migration.migrate().expect("segunda");
        assert!(!outcome.mode_changed);
        assert_eq!(
            store.load().expect("lê").assistant_mode_id,
            LEGACY_DEFAULT_MODE_ID
        );
    }

    #[test]
    fn a_custom_agent_is_never_touched() {
        let settings = Settings {
            assistant_mode_id: "custom-vendas".to_string(),
            custom_assistant_modes: vec![crate::domain::assistant::AssistantMode {
                id: "custom-vendas".into(),
                name: "Vendas".into(),
                category: "custom".into(),
                system_prompt: "Você ajuda em vendas.".into(),
                suggestion_prompt: "Sugira a próxima fala.".into(),
                summary_prompt: String::new(),
                context: String::new(),
                icon: "briefcase".into(),
                color: "coral".into(),
                source_id: None,
            }],
            ..Settings::default()
        };
        let store = Arc::new(InMemorySettingsStore::with_settings(settings));
        let migration = AssistantModeMigration::new(store.clone());

        let outcome = migration.migrate().expect("migra");
        assert!(!outcome.mode_changed);
        assert_eq!(store.load().expect("lê").assistant_mode_id, "custom-vendas");
    }

    #[test]
    fn the_migration_is_written_to_the_settings_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");

        // Um arquivo antigo (pré-BR-059): a "Reunião" era o padrão e a lista de
        // agentes do usuário ainda não existia.
        let mut legacy =
            serde_json::to_value(crate::domain::settings::Settings::default()).expect("json");
        legacy["assistant_mode_id"] = serde_json::json!(LEGACY_DEFAULT_MODE_ID);
        legacy["assistant_mode_migrated"] = serde_json::json!(false);
        std::fs::write(&path, serde_json::to_string(&legacy).expect("json")).expect("escreve");

        let store = Arc::new(FileSettingsStore::new(path.clone()));
        AssistantModeMigration::new(store.clone())
            .migrate()
            .expect("migra");

        let raw = std::fs::read_to_string(&path).expect("lê");
        assert!(raw.contains(DEFAULT_MODE_ID));
        assert!(raw.contains("assistant_mode_migrated"));
    }
}
