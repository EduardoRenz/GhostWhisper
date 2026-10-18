//! Porta de persistência do resumo de uma sessão.

use crate::domain::overview::MeetingSummary;
use crate::error::Result;

/// Guarda o resumo (documento Markdown) e o cursor incremental de cada sessão.
///
/// Adaptador: `LocalMeetingSummariesRepository`; dobra:
/// `InMemoryMeetingSummariesRepository`.
pub trait MeetingSummariesRepository: Send + Sync {
    /// Resumo gravado da sessão, se existir.
    fn get(&self, session_id: &str) -> Result<Option<MeetingSummary>>;
    /// Insere ou substitui o resumo da sessão.
    fn save(&self, summary: &MeetingSummary) -> Result<()>;
}
