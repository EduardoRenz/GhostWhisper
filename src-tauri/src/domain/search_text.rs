//! Normalização de texto para busca (BR-103).
//!
//! Regras **puras**, compartilhadas pela dobra em memória e pela busca SQLite
//! (a função `ghw_fold` registrada na conexão chama [`fold_text`]). A busca é
//! insensível a caixa e a acentos: "reuniao" encontra "Reunião".

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// Dobra o texto para comparação: minúsculas e **sem diacríticos**.
pub fn fold_text(text: &str) -> String {
    text.nfd()
        .filter(|character| !is_combining_mark(*character))
        .flat_map(|character| character.to_lowercase())
        .collect()
}

/// Tokens não vazios da consulta, já dobrados (AND entre eles).
pub fn query_tokens(query: &str) -> Vec<String> {
    fold_text(query)
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// Padrão de [`LIKE`] (`%…%`) com os curingas do usuário escapados.
///
/// Usado com `ESCAPE '\'`: `%`, `_` e `\` digitados viram texto literal.
pub fn like_pattern(token: &str) -> String {
    let escaped = token
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_removes_accents_and_lowercases() {
        assert_eq!(fold_text("Reunião de Sexta"), "reuniao de sexta");
        assert_eq!(fold_text("AÇÃO/Âmbar"), "acao/ambar");
        assert_eq!(fold_text("über"), "uber");
    }

    #[test]
    fn tokens_split_and_fold_every_word() {
        assert_eq!(query_tokens("  Reunião   ção  "), vec!["reuniao", "cao"]);
        assert!(query_tokens("   ").is_empty());
    }

    #[test]
    fn folded_title_contains_every_folded_token() {
        let tokens = query_tokens("reuniao LEDGER");
        let folded = fold_text("Reunião do ledger bancário");
        assert!(tokens.iter().all(|token| folded.contains(token.as_str())));

        let folded = fold_text("Reunião de sexta");
        assert!(!tokens.iter().all(|token| folded.contains(token.as_str())));
    }

    #[test]
    fn like_pattern_escapes_wildcards() {
        assert_eq!(like_pattern("100%"), "%100\\%%");
        assert_eq!(like_pattern("a_b"), "%a\\_b%");
        assert_eq!(like_pattern("c\\d"), "%c\\\\d%");
    }
}
