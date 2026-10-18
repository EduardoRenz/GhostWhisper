//! Resolução da sessão alvo de resumo/sugestão (helper da aplicação).

use crate::domain::session::Session;
use crate::error::{GhostWhisperError, Result};
use crate::ports::repositories::SessionRepository;

/// Resolve a sessão pedida ou, sem id, a mais recente.
///
/// A sessão **ativa** (gravando/pausada) é sempre a mais recente por
/// `started_at`, então `list_recent(1)` a prefere naturalmente.
pub fn resolve_session(
    sessions: &dyn SessionRepository,
    session_id: Option<String>,
) -> Result<Session> {
    match session_id {
        Some(id) => sessions
            .get(&id)?
            .ok_or_else(|| GhostWhisperError::Config(format!("sessão {id} não encontrada"))),
        None => sessions
            .list_recent(1)?
            .into_iter()
            .next()
            .ok_or_else(|| GhostWhisperError::Config("nenhuma sessão disponível".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::memory::in_memory_session_repository::InMemorySessionRepository;
    use crate::domain::session::SessionStatus;

    fn session(id: &str, started_at_ms: i64) -> Session {
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

    #[test]
    fn resolves_by_id_or_most_recent() {
        let repo = InMemorySessionRepository::new();
        repo.create(&session("a", 100)).unwrap();
        repo.create(&session("b", 200)).unwrap();

        assert_eq!(
            resolve_session(&repo, Some("a".to_string()))
                .expect("por id")
                .id,
            "a"
        );
        assert_eq!(resolve_session(&repo, None).expect("recente").id, "b");
    }

    #[test]
    fn errors_for_unknown_or_missing_session() {
        let repo = InMemorySessionRepository::new();
        assert!(resolve_session(&repo, None).is_err());
        assert!(resolve_session(&repo, Some("x".to_string())).is_err());
    }
}
