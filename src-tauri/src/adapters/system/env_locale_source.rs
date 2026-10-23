//! Locale do sistema via variáveis de ambiente (Linux-first).
//!
//! A ordem segue a convenção POSIX/glibc: `LC_ALL`, `LC_MESSAGES`, `LANG` e,
//! por último, `LANGUAGE` (lista separada por `:`). Valores `C`/`POSIX` não
//! trazem informação de idioma e são ignorados.

use crate::ports::locale_source::LocaleSource;

/// Lê o locale preferido das variáveis de ambiente.
pub struct EnvLocaleSource;

impl LocaleSource for EnvLocaleSource {
    fn system_locale(&self) -> Option<String> {
        let values = [
            std::env::var("LC_ALL").ok(),
            std::env::var("LC_MESSAGES").ok(),
            std::env::var("LANG").ok(),
            std::env::var("LANGUAGE").ok(),
        ];
        pick_locale(values.iter().map(|value| value.as_deref()))
    }
}

/// Primeiro locale utilizável da lista (puro, para teste).
fn pick_locale<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<String> {
    for value in values.into_iter().flatten() {
        for candidate in value.split(':') {
            let candidate = candidate.trim();
            if candidate.is_empty()
                || candidate.eq_ignore_ascii_case("c")
                || candidate.eq_ignore_ascii_case("posix")
            {
                continue;
            }
            return Some(candidate.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_first_usable_locale() {
        let locale = pick_locale([None, Some(""), Some("pt_BR.UTF-8"), Some("en_US")]);
        assert_eq!(locale.as_deref(), Some("pt_BR.UTF-8"));
    }

    #[test]
    fn skips_c_and_posix() {
        let locale = pick_locale([Some("C"), Some("POSIX"), Some("en-US")]);
        assert_eq!(locale.as_deref(), Some("en-US"));
        assert_eq!(pick_locale([Some("C.UTF-8")]).as_deref(), Some("C.UTF-8"));
        assert_eq!(pick_locale([Some("C"), Some("POSIX")]), None);
    }

    #[test]
    fn supports_language_lists() {
        let locale = pick_locale([Some("pt_BR:en_US")]);
        assert_eq!(locale.as_deref(), Some("pt_BR"));
    }
}
