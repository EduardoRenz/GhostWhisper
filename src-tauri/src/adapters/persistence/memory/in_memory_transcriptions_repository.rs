//! Dobra de teste para `TranscriptionsRepository`.

use std::sync::Mutex;

use crate::domain::transcription::TranscriptSegment;
use crate::error::{GhostWhisperError, Result};
use crate::ports::repositories::TranscriptionsRepository;

/// Repositório de transcrições em memória (busca por substring).
#[derive(Default)]
pub struct InMemoryTranscriptionsRepository {
    segments: Mutex<Vec<TranscriptSegment>>,
}

impl InMemoryTranscriptionsRepository {
    /// Cria um repositório vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retorna uma cópia de todos os segmentos.
    pub fn all(&self) -> Vec<TranscriptSegment> {
        self.segments.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl TranscriptionsRepository for InMemoryTranscriptionsRepository {
    fn insert(&self, segment: &TranscriptSegment) -> Result<()> {
        self.segments
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de transcrições envenenado".to_string()))?
            .push(segment.clone());
        Ok(())
    }

    fn list_by_session(&self, session_id: &str) -> Result<Vec<TranscriptSegment>> {
        let mut segments: Vec<_> = self
            .all()
            .into_iter()
            .filter(|segment| segment.session_id == session_id)
            .collect();
        segments.sort_by_key(|segment| segment.start_ms);
        Ok(segments)
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<TranscriptSegment>> {
        let needle = query.to_lowercase();
        let mut found: Vec<_> = self
            .all()
            .into_iter()
            .filter(|segment| segment.text.to_lowercase().contains(&needle))
            .collect();
        found.sort_by_key(|segment| segment.start_ms);
        found.truncate(limit);
        Ok(found)
    }

    fn latest_final_end_ms(&self, session_id: &str) -> Result<Option<i64>> {
        Ok(self
            .all()
            .into_iter()
            .filter(|segment| {
                segment.session_id == session_id
                    && segment.is_final
                    && !segment.text.trim().is_empty()
            })
            .map(|segment| segment.end_ms)
            .max())
    }

    fn list_recent_by_session(
        &self,
        session_id: &str,
        since_ms: i64,
        limit: usize,
    ) -> Result<Vec<TranscriptSegment>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut segments: Vec<_> = self
            .all()
            .into_iter()
            .filter(|segment| {
                segment.session_id == session_id
                    && segment.is_final
                    && !segment.text.trim().is_empty()
                    && segment.end_ms >= since_ms
            })
            .collect();
        segments.sort_by_key(|segment| (segment.start_ms, segment.end_ms));
        let start = segments.len().saturating_sub(limit);
        Ok(segments.split_off(start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::transcription::Speaker;

    fn segment(id: &str, at: i64, text: &str, is_final: bool) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            session_id: "s1".to_string(),
            speaker: Speaker::Me,
            text: text.to_string(),
            start_ms: at,
            end_ms: at + 500,
            is_final,
        }
    }

    #[test]
    fn latest_final_end_ms_ignores_previews_and_empty_text() {
        let repo = InMemoryTranscriptionsRepository::new();
        repo.insert(&segment("1", 1_000, "final", true)).unwrap();
        repo.insert(&segment("2", 2_000, "prévia", false)).unwrap();
        repo.insert(&segment("3", 3_000, "  ", true)).unwrap();
        assert_eq!(repo.latest_final_end_ms("s1").expect("latest"), Some(1_500));
        assert_eq!(repo.latest_final_end_ms("s2").expect("latest"), None);
    }

    #[test]
    fn list_recent_filters_and_keeps_the_last_n_in_order() {
        let repo = InMemoryTranscriptionsRepository::new();
        repo.insert(&segment("1", 1_000, "primeira", true)).unwrap();
        repo.insert(&segment("2", 2_000, "segunda", true)).unwrap();
        repo.insert(&segment("3", 3_000, "prévia", false)).unwrap();
        repo.insert(&segment("4", 4_000, "   ", true)).unwrap();
        repo.insert(&segment("5", 5_000, "quinta", true)).unwrap();

        let recent = repo.list_recent_by_session("s1", 0, 2).expect("recent");
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, "2");
        assert_eq!(recent[1].id, "5");

        let since = repo.list_recent_by_session("s1", 2_501, 10).expect("since");
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].id, "5");

        assert!(repo
            .list_recent_by_session("s1", 0, 0)
            .expect("limit zero")
            .is_empty());
    }
}
