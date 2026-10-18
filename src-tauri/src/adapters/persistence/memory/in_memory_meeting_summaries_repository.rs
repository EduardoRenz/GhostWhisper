//! Dobra de teste para `MeetingSummariesRepository`.

use std::sync::Mutex;

use crate::domain::overview::MeetingSummary;
use crate::error::{GhostWhisperError, Result};
use crate::ports::meeting_summaries_repository::MeetingSummariesRepository;

/// Repositório de resumos em memória.
#[derive(Default)]
pub struct InMemoryMeetingSummariesRepository {
    summaries: Mutex<Vec<MeetingSummary>>,
}

impl InMemoryMeetingSummariesRepository {
    /// Cria um repositório vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retorna uma cópia de todos os resumos.
    pub fn all(&self) -> Vec<MeetingSummary> {
        self.summaries.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl MeetingSummariesRepository for InMemoryMeetingSummariesRepository {
    fn get(&self, session_id: &str) -> Result<Option<MeetingSummary>> {
        let summaries = self
            .summaries
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de resumos envenenado".to_string()))?;
        Ok(summaries
            .iter()
            .find(|summary| summary.session_id == session_id)
            .cloned())
    }

    fn save(&self, summary: &MeetingSummary) -> Result<()> {
        let mut summaries = self
            .summaries
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de resumos envenenado".to_string()))?;
        summaries.retain(|existing| existing.session_id != summary.session_id);
        summaries.push(summary.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(markdown: &str, until: i64) -> MeetingSummary {
        MeetingSummary {
            session_id: "s1".to_string(),
            subject: None,
            markdown: markdown.to_string(),
            summarized_from_ms: 0,
            summarized_until_ms: until,
            updated_at_ms: 0,
        }
    }

    #[test]
    fn saves_reads_and_replaces() {
        let repo = InMemoryMeetingSummariesRepository::new();
        assert!(repo.get("s1").expect("get").is_none());
        repo.save(&summary("a", 1)).expect("save");
        repo.save(&summary("b", 2)).expect("save");
        let stored = repo.get("s1").expect("get").expect("existe");
        assert_eq!(stored.markdown, "b");
        assert_eq!(repo.all().len(), 1);
    }
}
