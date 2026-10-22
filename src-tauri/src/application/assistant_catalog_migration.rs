//! Semeadura **única** dos agentes de fábrica (BR-075).
//!
//! O agente **Padrão** é protegido (BR-074) e vive no core. Os demais agentes
//! de fábrica (Reunião, Entrevista Técnica, LeetCode, System Design) são
//! **materializados** na lista do usuário uma única vez: a partir daí são
//! agentes dele — podem ser editados e excluídos de vez.
//!
//! A migração é **idempotente e única** (marca `assistant_modes_seeded`):
//! depois de ligada, um agente excluído não volta no próximo boot.

use std::sync::Arc;

use crate::application::prompts::assistant::factory_modes;
use crate::error::Result;
use crate::ports::settings_store::SettingsStore;

/// Resultado da semeadura (para o log do boot).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SeedOutcome {
    /// Ids dos agentes de fábrica materializados nesta execução.
    pub added: Vec<String>,
    /// A lista já está semeada.
    pub seeded: bool,
}

/// Materializa os agentes de fábrica na lista do usuário (uma vez só).
pub struct AssistantCatalogMigration {
    settings: Arc<dyn SettingsStore>,
}

impl AssistantCatalogMigration {
    /// Cria a migração com a porta resolvida.
    pub fn new(settings: Arc<dyn SettingsStore>) -> Self {
        Self { settings }
    }

    /// Executa a semeadura (idempotente).
    pub fn seed(&self) -> Result<SeedOutcome> {
        let mut settings = self.settings.load()?;
        if settings.assistant_modes_seeded {
            return Ok(SeedOutcome {
                added: Vec::new(),
                seeded: true,
            });
        }

        let mut added = Vec::new();
        for mode in factory_modes() {
            if !settings
                .custom_assistant_modes
                .iter()
                .any(|existing| existing.id == mode.id)
            {
                added.push(mode.id.clone());
                settings.custom_assistant_modes.push(mode);
            }
        }
        settings.assistant_modes_seeded = true;
        self.settings.save(&settings)?;
        Ok(SeedOutcome {
            added,
            seeded: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::domain::assistant::AssistantMode;
    use crate::domain::settings::Settings;

    fn store_with(settings: Settings) -> Arc<InMemorySettingsStore> {
        Arc::new(InMemorySettingsStore::with_settings(settings))
    }

    fn agent(id: &str, name: &str) -> AssistantMode {
        AssistantMode {
            id: id.into(),
            name: name.into(),
            category: "custom".into(),
            system_prompt: "Você ajuda.".into(),
            suggestion_prompt: "Sugira.".into(),
            summary_prompt: String::new(),
            context: String::new(),
            icon: "sparkle".into(),
            color: "slate".into(),
            source_id: None,
        }
    }

    #[test]
    fn seeds_the_four_factory_agents_once() {
        let store = store_with(Settings::default());
        let migration = AssistantCatalogMigration::new(store.clone());

        let outcome = migration.seed().expect("semeia");
        assert!(outcome.seeded);
        assert_eq!(
            outcome.added,
            vec![
                "meeting",
                "technical_interview",
                "leetcode",
                "system_design"
            ]
        );

        let settings = store.load().expect("lê");
        assert!(settings.assistant_modes_seeded);
        assert_eq!(settings.custom_assistant_modes.len(), 4);
    }

    #[test]
    fn seeding_twice_changes_nothing() {
        let store = store_with(Settings::default());
        let migration = AssistantCatalogMigration::new(store.clone());

        migration.seed().expect("primeira");
        let after_first = store.load().expect("lê");

        let outcome = migration.seed().expect("segunda");
        assert!(outcome.added.is_empty());
        assert_eq!(after_first, store.load().expect("lê"));
    }

    #[test]
    fn never_overwrites_a_user_agent_with_the_same_id() {
        let settings = Settings {
            custom_assistant_modes: vec![agent("meeting", "Reunião da empresa")],
            ..Settings::default()
        };
        let store = store_with(settings);
        let migration = AssistantCatalogMigration::new(store.clone());

        let outcome = migration.seed().expect("semeia");
        assert!(!outcome.added.contains(&"meeting".to_string()));

        let settings = store.load().expect("lê");
        let meeting = settings
            .custom_assistant_modes
            .iter()
            .find(|mode| mode.id == "meeting")
            .expect("reunião");
        assert_eq!(meeting.name, "Reunião da empresa");
        assert_eq!(settings.custom_assistant_modes.len(), 4);
    }

    #[test]
    fn a_deleted_agent_does_not_come_back() {
        let store = store_with(Settings::default());
        let migration = AssistantCatalogMigration::new(store.clone());
        migration.seed().expect("semeia");

        // O usuário exclui um agente de fábrica.
        let mut settings = store.load().expect("lê");
        settings
            .custom_assistant_modes
            .retain(|mode| mode.id != "leetcode");
        assert_eq!(settings.assistant_mode_id, "default");
        store.save(&settings).expect("salva");

        // Um novo boot não ressuscita o agente.
        migration.seed().expect("segunda");
        let settings = store.load().expect("lê");
        assert!(!settings
            .custom_assistant_modes
            .iter()
            .any(|mode| mode.id == "leetcode"));
    }

    #[test]
    fn the_selected_factory_agent_stays_selected() {
        let settings = Settings {
            assistant_mode_id: "leetcode".to_string(),
            ..Settings::default()
        };
        let store = store_with(settings);
        AssistantCatalogMigration::new(store.clone())
            .seed()
            .expect("semeia");

        let settings = store.load().expect("lê");
        assert_eq!(settings.assistant_mode_id, "leetcode");
        assert!(settings.selected_mode_exists());
    }
}
