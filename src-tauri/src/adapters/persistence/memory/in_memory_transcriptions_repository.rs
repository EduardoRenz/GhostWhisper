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
}
