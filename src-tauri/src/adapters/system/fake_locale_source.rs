//! Dobra de teste da porta `LocaleSource`.

use crate::ports::locale_source::LocaleSource;

/// Locale fixo, definido pelo teste (ou ausente).
#[derive(Debug, Clone, Default)]
pub struct FakeLocaleSource {
    locale: Option<String>,
}

impl FakeLocaleSource {
    /// Fonte sem locale conhecido.
    pub fn none() -> Self {
        Self { locale: None }
    }

    /// Fonte com um locale fixo.
    pub fn with(locale: impl Into<String>) -> Self {
        Self {
            locale: Some(locale.into()),
        }
    }
}

impl LocaleSource for FakeLocaleSource {
    fn system_locale(&self) -> Option<String> {
        self.locale.clone()
    }
}
