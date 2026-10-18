//! Repositório SQLite do resumo de sessão.

use rusqlite::{params, OptionalExtension, Row};

use crate::domain::overview::MeetingSummary;
use crate::error::Result;
use crate::ports::meeting_summaries_repository::MeetingSummariesRepository;

use super::connection::{lock, SharedConnection};

const COLUMNS: &str =
    "session_id, subject, content, summarized_from_ms, summarized_until_ms, updated_at_ms";

/// Implementa `MeetingSummariesRepository` em SQLite.
pub struct LocalMeetingSummariesRepository {
    conn: SharedConnection,
}

impl LocalMeetingSummariesRepository {
    /// Cria o repositório sobre a conexão compartilhada.
    pub fn new(conn: SharedConnection) -> Self {
        Self { conn }
    }
}

impl MeetingSummariesRepository for LocalMeetingSummariesRepository {
    fn get(&self, session_id: &str) -> Result<Option<MeetingSummary>> {
        let conn = lock(&self.conn)?;
        let query = format!("SELECT {COLUMNS} FROM meeting_summaries WHERE session_id = ?1");
        let summary = conn
            .query_row(&query, params![session_id], map_summary)
            .optional()?;
        Ok(summary)
    }

    fn save(&self, summary: &MeetingSummary) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "INSERT INTO meeting_summaries \
             (session_id, subject, content, summarized_from_ms, summarized_until_ms, updated_at_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(session_id) DO UPDATE SET \
               subject = excluded.subject, \
               content = excluded.content, \
               summarized_from_ms = excluded.summarized_from_ms, \
               summarized_until_ms = excluded.summarized_until_ms, \
               updated_at_ms = excluded.updated_at_ms",
            params![
                summary.session_id,
                summary.subject,
                summary.markdown,
                summary.summarized_from_ms,
                summary.summarized_until_ms,
                summary.updated_at_ms,
            ],
        )?;
        Ok(())
    }
}

fn map_summary(row: &Row<'_>) -> rusqlite::Result<MeetingSummary> {
    Ok(MeetingSummary {
        session_id: row.get(0)?,
        subject: row.get(1)?,
        markdown: row.get(2)?,
        summarized_from_ms: row.get(3)?,
        summarized_until_ms: row.get(4)?,
        updated_at_ms: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::sqlite::local_session_repository::{
        tests_support, LocalSessionRepository,
    };
    use crate::ports::repositories::SessionRepository;

    fn repo() -> LocalMeetingSummariesRepository {
        let conn = tests_support::in_memory();
        let sessions = LocalSessionRepository::new(conn.clone());
        sessions.create(&tests_support::session("s1", 0)).unwrap();
        LocalMeetingSummariesRepository::new(conn)
    }

    fn summary(markdown: &str, until: i64) -> MeetingSummary {
        MeetingSummary {
            session_id: "s1".to_string(),
            subject: Some("Título".to_string()),
            markdown: markdown.to_string(),
            summarized_from_ms: 0,
            summarized_until_ms: until,
            updated_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn saves_reads_and_upserts() {
        let repo = repo();
        assert!(repo.get("s1").expect("get").is_none());

        repo.save(&summary("## A", 1_000)).expect("save");
        let stored = repo.get("s1").expect("get").expect("existe");
        assert_eq!(stored.markdown, "## A");
        assert_eq!(stored.subject.as_deref(), Some("Título"));

        repo.save(&summary("## B", 2_000)).expect("upsert");
        let stored = repo.get("s1").expect("get").expect("existe");
        assert_eq!(stored.markdown, "## B");
        assert_eq!(stored.summarized_until_ms, 2_000);
    }

    #[test]
    fn deleting_the_session_removes_the_summary() {
        let conn = tests_support::in_memory();
        let sessions = LocalSessionRepository::new(conn.clone());
        sessions.create(&tests_support::session("s1", 0)).unwrap();
        let repo = LocalMeetingSummariesRepository::new(conn);
        repo.save(&summary("## A", 1_000)).expect("save");

        sessions.delete("s1").expect("delete");

        assert!(repo.get("s1").expect("get").is_none());
    }
}
