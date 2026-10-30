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

    fn resume(&self, id: &str) -> Result<()> {
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(session) = sessions.iter_mut().find(|session| session.id == id) {
                session.ended_at_ms = None;
                session.status = SessionStatus::Active;
            }
        }
        Ok(())
    }

    fn end_all_active(&self, ended_at_ms: i64) -> Result<usize> {
        let mut ended = 0;
        if let Ok(mut sessions) = self.sessions.lock() {
            for session in sessions.iter_mut() {
                if session.status == SessionStatus::Active {
                    session.ended_at_ms = Some(ended_at_ms);
                    session.status = SessionStatus::Ended;
                    ended += 1;
                }
            }
        }
        Ok(ended)
    }

    fn rename(&self, id: &str, subject: &str) -> Result<()> {
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(session) = sessions.iter_mut().find(|session| session.id == id) {
                session.subject = Some(subject.to_string());
            }
        }
        Ok(())
    }

    fn set_overview(&self, id: &str, overview: &str) -> Result<()> {
        if let Ok(mut sessions) = self.sessions.lock() {
            if let Some(session) = sessions.iter_mut().find(|session| session.id == id) {
                session.overview = Some(overview.to_string());
            }
        }
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<()> {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.retain(|session| session.id != id);
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

    fn search_by_subject(&self, query: &str, limit: usize) -> Result<Vec<Session>> {
        let tokens = crate::domain::search_text::query_tokens(query);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let mut sessions: Vec<Session> = self
            .all()
            .into_iter()
            .filter(|session| {
                session.subject.as_deref().is_some_and(|subject| {
                    let folded = crate::domain::search_text::fold_text(subject);
                    tokens.iter().all(|token| folded.contains(token.as_str()))
                })
            })
            .collect();
        sessions.sort_by_key(|session| std::cmp::Reverse(session.started_at_ms));
        sessions.truncate(limit);
        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titled(id: &str, started_at_ms: i64, subject: Option<&str>) -> Session {
        Session {
            id: id.to_string(),
            started_at_ms,
            ended_at_ms: None,
            provider: "local".to_string(),
            model: None,
            transcription_model: None,
            user_id: None,
            subject: subject.map(str::to_string),
            overview: None,
            status: SessionStatus::Ended,
        }
    }

    #[test]
    fn search_by_subject_ignores_case_and_accents() {
        let repo = InMemorySessionRepository::new();
        repo.create(&titled("a", 100, Some("Reunião de sexta")))
            .expect("a");
        repo.create(&titled("b", 200, Some("Ledger bancário")))
            .expect("b");
        repo.create(&titled("c", 300, None)).expect("c");

        let found = repo.search_by_subject("REUNIAO", 10).expect("search");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "a");
    }

    #[test]
    fn search_by_subject_requires_every_word_and_orders_newest_first() {
        let repo = InMemorySessionRepository::new();
        repo.create(&titled("old", 100, Some("Reunião do ledger")))
            .expect("old");
        repo.create(&titled("new", 200, Some("Reunião do ledger")))
            .expect("new");
        repo.create(&titled("other", 300, Some("Reunião de sexta")))
            .expect("other");

        let found = repo
            .search_by_subject("reuniao ledger", 10)
            .expect("search");
        assert_eq!(
            found.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["new", "old"]
        );

        assert!(repo.search_by_subject("  ", 10).expect("empty").is_empty());
    }
}
