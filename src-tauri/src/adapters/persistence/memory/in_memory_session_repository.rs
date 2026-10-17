//! Dobra de teste para `SessionRepository`.

use std::sync::Mutex;

use crate::domain::session::{Session, SessionStatus};
use crate::error::{GhostWhisperError, Result};
use crate::ports::repositories::SessionRepository;

/// Repositório de sessões em memória.
#[derive(Default)]
pub struct InMemorySessionRepository {
    sessions: Mutex<Vec<Session>>,
}

impl InMemorySessionRepository {
    /// Cria um repositório vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retorna uma cópia de todas as sessões.
    pub fn all(&self) -> Vec<Session> {
        self.sessions.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn find(&self, id: &str) -> Option<Session> {
        self.sessions
            .lock()
            .ok()?
            .iter()
            .find(|session| session.id == id)
            .cloned()
    }
}

impl SessionRepository for InMemorySessionRepository {
    fn create(&self, session: &Session) -> Result<()> {
        self.sessions
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de sessões envenenado".to_string()))?
            .push(session.clone());
        Ok(())
    }

    fn end(&self, id: &str, ended_at_ms: i64) -> Result<()> {
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(session) = sessions.iter_mut().find(|session| session.id == id) {
                session.ended_at_ms = Some(ended_at_ms);
                session.status = SessionStatus::Ended;
            }
        }
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<Session>> {
        Ok(self.find(id))
    }

    fn list_recent(&self, limit: usize) -> Result<Vec<Session>> {
        let mut sessions = self.all();
        sessions.sort_by_key(|session| std::cmp::Reverse(session.started_at_ms));
        sessions.truncate(limit);
        Ok(sessions)
    }
}
