//! Migração **única** das perguntas prontas (BR-054).
//!
//! A v6 acrescentou o preset **"Responda a última pergunta identificada"** —
//! um pedido, não uma pergunta ("responda à última pergunta que você
//! identificou"). Quem já tinha a lista persistida nas settings não a
//! receberia pelo default do serde, então a marca
//! (`saved_questions_migrated`) garante que ela entra **uma vez só** — e
//! **não** ressuscita presets que o usuário apagou.

use std::sync::Arc;

use crate::domain::settings::{MAX_SAVED_QUESTIONS, REPLY_TO_LAST_QUESTION, SavedQuestion};
use crate::error::Result;
use crate::ports::settings_store::SettingsStore;

/// Resultado da migração (para o log do boot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MigrationOutcome {
    /// O preset novo foi acrescentado à lista.
    pub question_added: bool,
    /// A migração rodou (a marca foi gravada).
    pub marked: bool,
}

/// Acrescenta o preset novo das perguntas prontas (uma vez só).
pub struct SavedQuestionsMigration {
    settings: Arc<dyn SettingsStore>,
}

impl SavedQuestionsMigration {
    /// Cria a migração com a porta resolvida.
    pub fn new(settings: Arc<dyn SettingsStore>) -> Self {
        Self { settings }
    }

    /// Executa a migração (idempotente).
    pub fn migrate(&self) -> Result<MigrationOutcome> {
        let mut settings = self.settings.load()?;
        if settings.saved_questions_migrated {
            return Ok(MigrationOutcome::default());
        }

        settings.saved_questions_migrated = true;
        let already_there = settings
            .saved_questions
            .iter()
            .any(|question| question.text.trim() == REPLY_TO_LAST_QUESTION);
        let mut question_added = false;
        if !already_there && settings.saved_questions.len() < MAX_SAVED_QUESTIONS {
            settings.saved_questions.push(SavedQuestion {
                id: next_default_id(&settings.saved_questions),
                text: REPLY_TO_LAST_QUESTION.to_string(),
            });
            question_added = true;
        }
        self.settings.save(&settings)?;
        Ok(MigrationOutcome {
            question_added,
            marked: true,
        })
    }
}

/// Id `default-N` livre (o mesmo esquema dos presets semeados).
fn next_default_id(existing: &[SavedQuestion]) -> String {
    let mut index = existing.len();
    loop {
        let id = format!("default-{index}");
        if !existing.iter().any(|question| question.id == id) {
            return id;
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use crate::domain::settings::Settings;

    /// Store em memória: a migração lê e grava o mesmo objeto.
    struct MemoryStore {
        settings: Mutex<Settings>,
        saves: Mutex<usize>,
    }

    impl MemoryStore {
        fn with(settings: Settings) -> Self {
            Self {
                settings: Mutex::new(settings),
                saves: Mutex::new(0),
            }
        }

        fn saved(&self) -> Settings {
            self.settings.lock().expect("lock").clone()
        }

        fn saves(&self) -> usize {
            *self.saves.lock().expect("lock")
        }
    }

    impl SettingsStore for MemoryStore {
        fn load(&self) -> Result<Settings> {
            Ok(self.settings.lock().expect("lock").clone())
        }
        fn save(&self, settings: &Settings) -> Result<()> {
            *self.settings.lock().expect("lock") = settings.clone();
            *self.saves.lock().expect("lock") += 1;
            Ok(())
        }
    }

    /// Settings de quem já usava o app: as quatro perguntas antigas.
    fn old_settings() -> Settings {
        let mut settings = Settings::default();
        settings.saved_questions = crate::domain::settings::DEFAULT_SAVED_QUESTIONS[..4]
            .iter()
            .enumerate()
            .map(|(index, text)| SavedQuestion {
                id: format!("default-{index}"),
                text: (*text).to_string(),
            })
            .collect();
        settings.saved_questions_migrated = false;
        settings
    }

    #[test]
    fn adds_the_new_preset_once() {
        let store = Arc::new(MemoryStore::with(old_settings()));
        let migration = SavedQuestionsMigration::new(store.clone());

        let outcome = migration.migrate().expect("migra");
        assert!(outcome.question_added);
        assert!(outcome.marked);

        let saved = store.saved();
        assert_eq!(saved.saved_questions.len(), 5);
        let last = saved.saved_questions.last().expect("última");
        assert_eq!(last.text, REPLY_TO_LAST_QUESTION);
        assert!(saved.saved_questions_migrated);

        // A marca impede a segunda passada.
        let again = migration.migrate().expect("idempotente");
        assert!(!again.question_added);
        assert!(!again.marked);
        assert_eq!(store.saves(), 1, "não grava de novo");
    }

    #[test]
    fn does_not_duplicate_when_the_user_already_has_it() {
        let mut settings = old_settings();
        settings.saved_questions.push(SavedQuestion {
            id: "minha".into(),
            text: REPLY_TO_LAST_QUESTION.into(),
        });
        let store = Arc::new(MemoryStore::with(settings));

        let outcome = SavedQuestionsMigration::new(store.clone())
            .migrate()
            .expect("migra");

        assert!(!outcome.question_added);
        assert_eq!(store.saved().saved_questions.len(), 5);
    }

    #[test]
    fn respects_the_ceiling() {
        let mut settings = old_settings();
        while settings.saved_questions.len() < MAX_SAVED_QUESTIONS {
            let index = settings.saved_questions.len();
            settings.saved_questions.push(SavedQuestion {
                id: format!("q-{index}"),
                text: format!("pergunta {index}"),
            });
        }
        let store = Arc::new(MemoryStore::with(settings));

        let outcome = SavedQuestionsMigration::new(store.clone())
            .migrate()
            .expect("migra");

        assert!(!outcome.question_added, "lista cheia: nada é acrescentado");
        assert_eq!(
            store.saved().saved_questions.len(),
            MAX_SAVED_QUESTIONS
        );
    }

    #[test]
    fn a_fresh_install_already_has_it() {
        // Instalação nova: o default já traz o preset; a marca não o duplica.
        let store = Arc::new(MemoryStore::with(Settings::default()));
        let outcome = SavedQuestionsMigration::new(store.clone())
            .migrate()
            .expect("migra");

        assert!(!outcome.question_added);
        assert_eq!(store.saved().saved_questions.len(), 5);
        assert_eq!(
            store.saved().saved_questions.last().expect("última").text,
            REPLY_TO_LAST_QUESTION
        );
    }

    #[test]
    fn does_not_resurrect_a_deleted_preset() {
        let mut settings = old_settings();
        // O usuário apagou "O que posso falar?" antes de atualizar.
        settings.saved_questions.retain(|question| {
            question.text != "O que posso falar?"
        });
        let store = Arc::new(MemoryStore::with(settings));

        SavedQuestionsMigration::new(store.clone())
            .migrate()
            .expect("migra");

        let saved = store.saved();
        assert!(
            !saved
                .saved_questions
                .iter()
                .any(|question| question.text == "O que posso falar?"),
            "não ressuscita o que foi apagado"
        );
        assert_eq!(saved.saved_questions.len(), 4);
    }

    #[test]
    fn picks_a_free_default_id() {
        let existing = vec![SavedQuestion {
            id: "default-3".into(),
            text: "x".into(),
        }];
        // `default-1` é o primeiro livre a partir do tamanho da lista.
        assert_eq!(next_default_id(&existing), "default-1");
    }
}
