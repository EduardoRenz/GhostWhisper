//! Resumo persistido de uma sessão (documento Markdown gerado por LLM).
//!
//! É separado de [`crate::domain::session::Session`] porque carrega o **cursor**
//! de atualização incremental (`summarized_until_ms`) e o início coberto
//! (`summarized_from_ms`).

use serde::Serialize;

/// Resumo de uma sessão.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MeetingSummary {
    pub session_id: String,
    /// Título sugerido pelo modelo (pode ser diferente do título do usuário).
    pub subject: Option<String>,
    /// Corpo em Markdown.
    pub markdown: String,
    /// Primeiro instante da sessão coberto pelo resumo (ms desde o início).
    pub summarized_from_ms: i64,
    /// Cursor: último instante processado (ms desde o início).
    pub summarized_until_ms: i64,
    /// Relógio de parede da última atualização.
    pub updated_at_ms: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_with_the_fields_the_ui_reads() {
        let summary = MeetingSummary {
            session_id: "s1".into(),
            subject: Some("Planejamento".into()),
            markdown: "## Visão geral\n\nTudo certo.".into(),
            summarized_from_ms: 0,
            summarized_until_ms: 65_000,
            updated_at_ms: 1_700_000_000_000,
        };
        let json = serde_json::to_value(&summary).expect("json");
        assert_eq!(json["session_id"], "s1");
        assert_eq!(json["subject"], "Planejamento");
        assert_eq!(json["summarized_from_ms"], 0);
        assert_eq!(json["summarized_until_ms"], 65_000);
        assert!(json["markdown"].as_str().unwrap().contains("Visão geral"));
    }
}
