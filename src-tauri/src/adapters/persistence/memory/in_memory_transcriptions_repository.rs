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
}
