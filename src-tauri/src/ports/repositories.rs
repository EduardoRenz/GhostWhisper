//! Portas de persistência (sessões, transcrições e mensagens).

use crate::domain::session::Session;
use crate::domain::transcription::TranscriptSegment;
use crate::error::Result;

/// Persistência de sessões.
///
/// Adaptador: `LocalSessionRepository`; dobra: `InMemorySessionRepository`.
pub trait SessionRepository: Send + Sync {
    fn create(&self, session: &Session) -> Result<()>;
    fn end(&self, id: &str, ended_at_ms: i64) -> Result<()>;
    fn get(&self, id: &str) -> Result<Option<Session>>;
    fn list_recent(&self, limit: usize) -> Result<Vec<Session>>;
}

/// Persistência de segmentos de transcrição (com busca full-text).
///
/// Adaptador: `LocalTranscriptionsRepository`; dobra: `InMemoryTranscriptionsRepository`.
pub trait TranscriptionsRepository: Send + Sync {
    fn insert(&self, segment: &TranscriptSegment) -> Result<()>;
    fn list_by_session(&self, session_id: &str) -> Result<Vec<TranscriptSegment>>;
    fn search(&self, query: &str, limit: usize) -> Result<Vec<TranscriptSegment>>;
}
