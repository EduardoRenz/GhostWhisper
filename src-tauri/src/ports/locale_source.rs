//! Locale do sistema operacional.
//!
//! Porta pequena: a aplicação só precisa saber o locale **uma vez** (primeira
//! inicialização) para escolher o idioma da interface.

/// Fonte do locale do sistema (ex.: `pt_BR.UTF-8`, `en-US`).
pub trait LocaleSource: Send + Sync {
    /// Locale preferido do sistema, quando conhecido.
    fn system_locale(&self) -> Option<String>;
}
