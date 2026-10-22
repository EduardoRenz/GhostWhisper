//! Dobra de teste do repositório de gravações (BR-013).

use std::sync::Mutex;

use crate::domain::recording::{Recording, RecordingStatus};
use crate::error::{GhostWhisperError, Result};
use crate::ports::recordings_repository::RecordingsRepository;

/// Guarda as gravações em memória.
#[derive(Default)]
pub struct InMemoryRecordingsRepository {
    items: Mutex<Vec<Recording>>,
}

impl InMemoryRecordingsRepository {
    /// Cria o repositório vazio.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Vec<Recording>>> {
        self.items
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock das gravações envenenado".to_string()))
    }
}

impl RecordingsRepository for InMemoryRecordingsRepository {
    fn upsert(&self, recording: &Recording) -> Result<()> {
        let mut items = self.lock()?;
        match items.iter_mut().find(|item| item.id == recording.id) {
            Some(slot) => *slot = recording.clone(),
            None => items.push(recording.clone()),
        }
        Ok(())
    }

    fn update_status(&self, id: &str, status: RecordingStatus, error: Option<&str>) -> Result<()> {
        let mut items = self.lock()?;
        if let Some(item) = items.iter_mut().find(|item| item.id == id) {
            item.status = status;
            item.error = error.map(str::to_string);
        }
        Ok(())
    }

    fn set_artifact(
        &self,
        id: &str,
        path: &str,
        size_bytes: u64,
        duration_ms: i64,
        finished_at_ms: i64,
    ) -> Result<()> {
        let mut items = self.lock()?;
        if let Some(item) = items.iter_mut().find(|item| item.id == id) {
            item.path = Some(path.to_string());
            item.size_bytes = size_bytes;
            item.duration_ms = duration_ms;
            item.finished_at_ms = Some(finished_at_ms);
            item.status = RecordingStatus::Ready;
            item.error = None;
        }
        Ok(())
    }

    fn get_by_session(&self, session_id: &str) -> Result<Option<Recording>> {
        Ok(self
            .lock()?
            .iter()
            .filter(|item| item.session_id == session_id)
            .max_by_key(|item| item.started_at_ms)
            .cloned())
    }

    fn list_by_session(&self, session_id: &str) -> Result<Vec<Recording>> {
        let mut items: Vec<Recording> = self
            .lock()?
            .iter()
            .filter(|item| item.session_id == session_id)
            .cloned()
            .collect();
        items.sort_by_key(|item| item.started_at_ms);
        Ok(items)
    }

    fn list_unfinished(&self) -> Result<Vec<Recording>> {
        Ok(self
            .lock()?
            .iter()
            .filter(|item| {
                matches!(
                    item.status,
                    RecordingStatus::Recording | RecordingStatus::Paused
                )
            })
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recording::RecordingKind;

    fn recording() -> Recording {
        Recording {
            id: "r1".to_string(),
            session_id: "s1".to_string(),
            kind: RecordingKind::Video,
            status: RecordingStatus::Recording,
            path: None,
            segments_dir: None,
            size_bytes: 0,
            duration_ms: 0,
            width: 0,
            height: 0,
            fps: 8,
            started_at_ms: 0,
            finished_at_ms: None,
            error: None,
        }
    }

    #[test]
    fn upsert_updates_in_place() {
        let repo = InMemoryRecordingsRepository::new();
        repo.upsert(&recording()).expect("insert");
        let mut updated = recording();
        updated.status = RecordingStatus::Paused;
        repo.upsert(&updated).expect("update");
        let stored = repo.get_by_session("s1").expect("get").expect("existe");
        assert_eq!(stored.status, RecordingStatus::Paused);
        assert_eq!(repo.list_unfinished().expect("lista").len(), 1);
    }

    #[test]
    fn artifact_marks_ready() {
        let repo = InMemoryRecordingsRepository::new();
        repo.upsert(&recording()).expect("insert");
        repo.set_artifact("r1", "/tmp/a.mp4", 10, 20, 30)
            .expect("artifact");
        let stored = repo.get_by_session("s1").expect("get").expect("existe");
        assert_eq!(stored.status, RecordingStatus::Ready);
        assert_eq!(stored.path.as_deref(), Some("/tmp/a.mp4"));
        assert!(repo.list_unfinished().expect("lista").is_empty());
    }
}
