//! Repositório SQLite de sessões.

use rusqlite::{params, Row};

use crate::domain::session::{Session, SessionStatus};
use crate::error::Result;
use crate::ports::repositories::SessionRepository;

use super::connection::{lock, SharedConnection};

const COLUMNS: &str = "id, started_at, ended_at, provider, model, transcription_model, user_id, subject, overview, status";

/// Implementa `SessionRepository` em SQLite.
pub struct LocalSessionRepository {
    conn: SharedConnection,
}

impl LocalSessionRepository {
    /// Cria o repositório sobre a conexão compartilhada.
    pub fn new(conn: SharedConnection) -> Self {
        Self { conn }
    }
}

impl SessionRepository for LocalSessionRepository {
    fn create(&self, session: &Session) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "INSERT INTO sessions (id, started_at, ended_at, provider, model, transcription_model, user_id, subject, overview, status) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                session.id,
                session.started_at_ms,
                session.ended_at_ms,
                session.provider,
                session.model,
                session.transcription_model,
                session.user_id,
                session.subject,
                session.overview,
                session.status.as_str(),
            ],
        )?;
        Ok(())
    }

    fn end(&self, id: &str, ended_at_ms: i64) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "UPDATE sessions SET ended_at = ?2, status = 'ended' WHERE id = ?1",
            params![id, ended_at_ms],
        )?;
        Ok(())
    }

    fn resume(&self, id: &str) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "UPDATE sessions SET status = 'active', ended_at = NULL WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    fn end_all_active(&self, ended_at_ms: i64) -> Result<usize> {
        let conn = lock(&self.conn)?;
        let ended = conn.execute(
            "UPDATE sessions SET ended_at = ?1, status = 'ended' WHERE status = 'active'",
            params![ended_at_ms],
        )?;
        Ok(ended)
    }

    fn rename(&self, id: &str, subject: &str) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "UPDATE sessions SET subject = ?2 WHERE id = ?1",
            params![id, subject],
        )?;
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<()> {
        let conn = lock(&self.conn)?;
        // O `ON DELETE CASCADE` do schema já resolve, mas removemos os filhos
        // explicitamente para não depender do pragma estar ligado.
        conn.execute(
            "DELETE FROM transcriptions WHERE session_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM messages WHERE session_id = ?1", params![id])?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<Session>> {
        let conn = lock(&self.conn)?;
        let query = format!("SELECT {COLUMNS} FROM sessions WHERE id = ?1");
        let session = conn.query_row(&query, params![id], map_session).ok();
        Ok(session)
    }

    fn list_recent(&self, limit: usize) -> Result<Vec<Session>> {
        let conn = lock(&self.conn)?;
        let query = format!("SELECT {COLUMNS} FROM sessions ORDER BY started_at DESC LIMIT ?1");
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![limit as i64], map_session)?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }
}

fn map_session(row: &Row<'_>) -> rusqlite::Result<Session> {
    let status: String = row.get(9)?;
    Ok(Session {
        id: row.get(0)?,
        started_at_ms: row.get(1)?,
        ended_at_ms: row.get(2)?,
        provider: row.get(3)?,
        model: row.get(4)?,
        transcription_model: row.get(5)?,
        user_id: row.get(6)?,
        subject: row.get(7)?,
        overview: row.get(8)?,
        status: SessionStatus::from_str(&status).unwrap_or(SessionStatus::Active),
    })
}

#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;
    use crate::adapters::persistence::sqlite::connection;

    pub(crate) fn session(id: &str, started_at_ms: i64) -> Session {
        Session {
            id: id.to_string(),
            started_at_ms,
            ended_at_ms: None,
            provider: "local".to_string(),
            model: None,
            transcription_model: None,
            user_id: None,
            subject: None,
            overview: None,
            status: SessionStatus::Active,
        }
    }

    pub(crate) fn in_memory() -> SharedConnection {
        connection::shared(connection::open_in_memory().expect("in-memory"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_get_end_and_list() {
        let repo = LocalSessionRepository::new(tests_support::in_memory());
        repo.create(&tests_support::session("a", 100))
            .expect("create a");
        repo.create(&tests_support::session("b", 200))
            .expect("create b");

        assert_eq!(
            repo.get("a").unwrap().unwrap().status,
            SessionStatus::Active
        );

        repo.end("a", 150).expect("end a");
        let stored = repo.get("a").unwrap().unwrap();
        assert_eq!(stored.status, SessionStatus::Ended);
        assert_eq!(stored.ended_at_ms, Some(150));

        let recent = repo.list_recent(10).expect("list");
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, "b");
        assert_eq!(recent[1].id, "a");

        assert!(repo.get("missing").unwrap().is_none());
    }

    #[test]
    fn resume_reactivates_and_clears_ended_at() {
        let repo = LocalSessionRepository::new(tests_support::in_memory());
        repo.create(&tests_support::session("a", 100))
            .expect("create");
        repo.end("a", 150).expect("end");

        repo.resume("a").expect("resume");

        let stored = repo.get("a").unwrap().unwrap();
        assert_eq!(stored.status, SessionStatus::Active);
        assert_eq!(stored.ended_at_ms, None);
        assert_eq!(stored.started_at_ms, 100, "início preservado (BR-019)");
    }

    #[test]
    fn rename_sets_the_subject() {
        let repo = LocalSessionRepository::new(tests_support::in_memory());
        repo.create(&tests_support::session("a", 100))
            .expect("create");

        repo.rename("a", "Reunião de sexta").expect("rename");

        assert_eq!(
            repo.get("a").unwrap().unwrap().subject.as_deref(),
            Some("Reunião de sexta")
        );
    }

    #[test]
    fn delete_removes_the_session_and_its_children() {
        use crate::adapters::persistence::sqlite::local_messages_repository::LocalMessagesRepository;
        use crate::adapters::persistence::sqlite::local_transcriptions_repository::LocalTranscriptionsRepository;
        use crate::domain::message::{ConversationMessage, MessageRole};
        use crate::domain::transcription::{Speaker, TranscriptSegment};
        use crate::ports::repositories::{MessagesRepository, TranscriptionsRepository};

        let conn = tests_support::in_memory();
        let repo = LocalSessionRepository::new(conn.clone());
        let transcriptions = LocalTranscriptionsRepository::new(conn.clone());
        let messages = LocalMessagesRepository::new(conn.clone());
        repo.create(&tests_support::session("a", 100))
            .expect("create");

        transcriptions
            .insert(&TranscriptSegment {
                id: "seg".to_string(),
                session_id: "a".to_string(),
                speaker: Speaker::Me,
                text: "fala".to_string(),
                start_ms: 0,
                end_ms: 500,
                is_final: true,
            })
            .expect("insert segment");
        messages
            .insert(&ConversationMessage {
                id: "msg".to_string(),
                session_id: "a".to_string(),
                role: MessageRole::Assistant,
                content: "sugestão".to_string(),
                timestamp_ms: 0,
            })
            .expect("insert message");

        repo.delete("a").expect("delete");

        assert!(repo.get("a").expect("get").is_none());
        assert!(transcriptions
            .list_by_session("a")
            .expect("list")
            .is_empty());
        assert!(messages.list_by_session("a").expect("list").is_empty());
        // O índice FTS não pode sobrar (a busca devolveria fantasma).
        assert!(transcriptions
            .search("fala", 10)
            .expect("search")
            .is_empty());
    }

    #[test]
    fn end_all_active_closes_only_orphans_and_reports_how_many() {
        let repo = LocalSessionRepository::new(tests_support::in_memory());
        repo.create(&tests_support::session("a", 100))
            .expect("create a");
        repo.create(&tests_support::session("b", 200))
            .expect("create b");
        repo.create(&tests_support::session("c", 300))
            .expect("create c");
        repo.end("b", 250).expect("end b");

        let ended = repo.end_all_active(999).expect("end all");

        assert_eq!(ended, 2, "só as ativas");
        assert_eq!(repo.get("a").unwrap().unwrap().status, SessionStatus::Ended);
        assert_eq!(repo.get("a").unwrap().unwrap().ended_at_ms, Some(999));
        assert_eq!(
            repo.get("b").unwrap().unwrap().ended_at_ms,
            Some(250),
            "quem já tinha terminado não é reescrito"
        );
        assert_eq!(repo.get("c").unwrap().unwrap().status, SessionStatus::Ended);
    }
}
