//! Repositório SQLite de mensagens (sugestões da IA).
//!
//! Usa a tabela `messages` do schema original (task 05), com os triggers de FTS5
//! já existentes — **não** foi preciso migração.

use rusqlite::{params, Row};

use crate::domain::message::{ConversationMessage, MessageRole};
use crate::error::Result;
use crate::ports::repositories::MessagesRepository;

use super::connection::{lock, SharedConnection};

const COLUMNS: &str = "id, session_id, role, content, timestamp";

/// Implementa `MessagesRepository` em SQLite.
pub struct LocalMessagesRepository {
    conn: SharedConnection,
}

impl LocalMessagesRepository {
    /// Cria o repositório sobre a conexão compartilhada.
    pub fn new(conn: SharedConnection) -> Self {
        Self { conn }
    }
}

impl MessagesRepository for LocalMessagesRepository {
    fn insert(&self, message: &ConversationMessage) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "INSERT INTO messages (id, session_id, role, content, timestamp) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                message.id,
                message.session_id,
                message.role.as_str(),
                message.content,
                message.timestamp_ms,
            ],
        )?;
        Ok(())
    }

    fn list_by_session(&self, session_id: &str) -> Result<Vec<ConversationMessage>> {
        let conn = lock(&self.conn)?;
        let query =
            format!("SELECT {COLUMNS} FROM messages WHERE session_id = ?1 ORDER BY timestamp ASC");
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![session_id], map_message)?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }
}

fn map_message(row: &Row<'_>) -> rusqlite::Result<ConversationMessage> {
    let role: String = row.get(2)?;
    Ok(ConversationMessage {
        id: row.get(0)?,
        session_id: row.get(1)?,
        role: MessageRole::from_str(&role).unwrap_or(MessageRole::Assistant),
        content: row.get(3)?,
        timestamp_ms: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::sqlite::local_session_repository::{
        tests_support, LocalSessionRepository,
    };
    use crate::ports::repositories::SessionRepository;

    fn repo() -> LocalMessagesRepository {
        let conn = tests_support::in_memory();
        let sessions = LocalSessionRepository::new(conn.clone());
        sessions.create(&tests_support::session("s1", 0)).unwrap();
        LocalMessagesRepository::new(conn)
    }

    fn message(id: &str, at: i64, content: &str) -> ConversationMessage {
        ConversationMessage {
            id: id.to_string(),
            session_id: "s1".to_string(),
            role: MessageRole::Assistant,
            content: content.to_string(),
            timestamp_ms: at,
        }
    }

    #[test]
    fn insert_and_list_in_time_order() {
        let repo = repo();
        repo.insert(&message("2", 5_000, "segunda"))
            .expect("insert");
        repo.insert(&message("1", 1_000, "primeira"))
            .expect("insert");

        let listed = repo.list_by_session("s1").expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].content, "primeira");
        assert_eq!(listed[0].role, MessageRole::Assistant);
        assert_eq!(listed[1].timestamp_ms, 5_000);
    }

    #[test]
    fn ignores_other_sessions() {
        let repo = repo();
        repo.insert(&message("1", 1_000, "minha")).expect("insert");
        assert!(repo.list_by_session("s2").expect("list").is_empty());
    }
}
