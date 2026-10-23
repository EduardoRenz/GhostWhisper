//! Resolve o idioma da interface **uma única vez** (BR-085).
//!
//! Na primeira inicialização (`ui_language = None`), o locale do sistema decide
//! entre português e inglês e a escolha é persistida. Depois disso, o campo é
//! propriedade do usuário: nenhum boot volta a detectar ou sobrescrever.

use std::sync::Arc;

use crate::domain::settings::{is_supported_ui_language, ui_language_from_locale};
use crate::error::Result;
use crate::ports::locale_source::LocaleSource;
use crate::ports::settings_store::SettingsStore;

/// Resultado da resolução (para o log do boot).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiLanguageOutcome {
    /// O idioma foi resolvido agora e gravado.
    pub resolved: bool,
    /// Idioma em uso depois da migração.
    pub language: Option<String>,
}

/// Resolve e persiste o idioma da interface na primeira inicialização.
pub struct UiLanguageMigration {
    settings: Arc<dyn SettingsStore>,
    locales: Arc<dyn LocaleSource>,
}

impl UiLanguageMigration {
    /// Cria a migração com as portas resolvidas.
    pub fn new(settings: Arc<dyn SettingsStore>, locales: Arc<dyn LocaleSource>) -> Self {
        Self { settings, locales }
    }

    /// Executa a resolução (idempotente).
    pub fn migrate(&self) -> Result<UiLanguageOutcome> {
        let mut settings = self.settings.load()?;
        if let Some(language) = settings.ui_language.as_deref() {
            if is_supported_ui_language(language) {
                return Ok(UiLanguageOutcome {
                    resolved: false,
                    language: Some(language.to_string()),
                });
            }
        }

        let locale = self.locales.system_locale();
        let language = ui_language_from_locale(locale.as_deref()).to_string();
        settings.ui_language = Some(language.clone());
        self.settings.save(&settings)?;
        Ok(UiLanguageOutcome {
            resolved: true,
            language: Some(language),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::adapters::system::fake_locale_source::FakeLocaleSource;
    use crate::domain::settings::Settings;

    /// Store em memória com contador de saves (a migração é única).
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

    fn migration(store: &Arc<MemoryStore>, locale: FakeLocaleSource) -> UiLanguageMigration {
        UiLanguageMigration::new(store.clone(), Arc::new(locale))
    }

    #[test]
    fn detects_portuguese_once_on_first_launch() {
        let store = Arc::new(MemoryStore::with(Settings::default()));
        let outcome = migration(&store, FakeLocaleSource::with("pt_BR.UTF-8"))
            .migrate()
            .expect("migra");

        assert!(outcome.resolved);
        assert_eq!(outcome.language.as_deref(), Some("pt"));
        assert_eq!(store.saved().ui_language.as_deref(), Some("pt"));
        assert_eq!(store.saves(), 1);
    }

    #[test]
    fn unknown_locale_falls_back_to_english() {
        let store = Arc::new(MemoryStore::with(Settings::default()));
        let outcome = migration(&store, FakeLocaleSource::with("es_ES.UTF-8"))
            .migrate()
            .expect("migra");

        assert_eq!(outcome.language.as_deref(), Some("en"));
        assert_eq!(store.saved().ui_language.as_deref(), Some("en"));
    }

    #[test]
    fn missing_locale_falls_back_to_english() {
        let store = Arc::new(MemoryStore::with(Settings::default()));
        let outcome = migration(&store, FakeLocaleSource::none())
            .migrate()
            .expect("migra");
        assert_eq!(outcome.language.as_deref(), Some("en"));
    }

    #[test]
    fn never_overrides_the_users_choice() {
        let settings = Settings {
            ui_language: Some("pt".into()),
            ..Settings::default()
        };
        let store = Arc::new(MemoryStore::with(settings));

        // Mesmo que o sistema esteja em inglês, a escolha do usuário vence.
        let outcome = migration(&store, FakeLocaleSource::with("en_US"))
            .migrate()
            .expect("migra");

        assert!(!outcome.resolved);
        assert_eq!(outcome.language.as_deref(), Some("pt"));
        assert_eq!(store.saves(), 0, "não regrava o que já está resolvido");
    }

    #[test]
    fn repairs_an_unsupported_persisted_value() {
        let settings = Settings {
            ui_language: Some("fr".into()),
            ..Settings::default()
        };
        let store = Arc::new(MemoryStore::with(settings));

        let outcome = migration(&store, FakeLocaleSource::with("en_US"))
            .migrate()
            .expect("migra");

        assert!(outcome.resolved);
        assert_eq!(store.saved().ui_language.as_deref(), Some("en"));
    }
}
