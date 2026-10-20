//! Utilidades do FTS5 (busca full-text).
//!
//! O texto digitado pelo usuário é **sintaxe** para o FTS5: aspas, `*`, `:` ou
//! `-` quebram a consulta e a busca inteira falha em silêncio (ISS-042 — o
//! Perssua passa a string crua e sofre com isso).
//!
//! Aqui a query é transformada em **termos literais** unidos por `AND`.

/// Escapa a query do usuário para o `MATCH` do FTS5.
///
/// Cada palavra vira um termo entre aspas (aspas internas duplicadas) e os
/// termos são unidos por `AND` — é o comportamento esperado de um campo de
/// busca ("como inverte" acha "como … inverte" em qualquer ordem).
///
/// `None` quando a query não tem palavra nenhuma (não há o que buscar).
pub fn escape_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        return None;
    }
    Some(terms.join(" AND "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_has_nothing_to_search() {
        assert_eq!(escape_query(""), None);
        assert_eq!(escape_query("   "), None);
    }

    #[test]
    fn words_become_literal_terms_joined_by_and() {
        assert_eq!(
            escape_query("como inverte"),
            Some("\"como\" AND \"inverte\"".to_string())
        );
    }

    #[test]
    fn fts_syntax_from_the_user_is_neutralized() {
        // Aspas, asterisco e dois-pontos viram texto literal, não sintaxe.
        assert_eq!(
            escape_query("\"aspas\""),
            Some("\"\"\"aspas\"\"\"".to_string())
        );
        assert_eq!(escape_query("a*b"), Some("\"a*b\"".to_string()));
        assert_eq!(escape_query("x:y"), Some("\"x:y\"".to_string()));
        assert_eq!(escape_query("a -b"), Some("\"a\" AND \"-b\"".to_string()));
    }

    #[test]
    fn extra_spaces_do_not_change_the_result() {
        assert_eq!(
            escape_query("  como   inverte  "),
            Some("\"como\" AND \"inverte\"".to_string())
        );
    }
}
