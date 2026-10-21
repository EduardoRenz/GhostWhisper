//! Porta de persistência da gravação da sessão (BR-065/BR-067).
//!
//! Adaptador: `LocalRecordingsRepository`; dobra:
//! `InMemoryRecordingsRepository`.

use crate::domain::recording::{Recording, RecordingStatus};
use crate::error::Result;

/// Persistência do artefato de gravação (uma linha por sessão).
pub trait RecordingsRepository: Send + Sync {
    /// Insere ou atualiza a gravação inteira.
    fn upsert(&self, recording: &Recording) -> Result<()>;

    /// Atualiza só o estado (e o erro, quando houver).
    fn update_status(
        &self,
        id: &str,
        status: RecordingStatus,
        error: Option<&str>,
    ) -> Result<()>;

    /// Grava o resultado da finalização.
    fn set_artifact(
        &self,
        id: &str,
        path: &str,
        size_bytes: u64,
        duration_ms: i64,
        finished_at_ms: i64,
    ) -> Result<()>;

    /// Gravação **mais recente** da sessão (o modal mostra a última).
    fn get_by_session(&self, session_id: &str) -> Result<Option<Recording>>;

    /// Todas as gravações da sessão (descarte e exclusão).
    fn list_by_session(&self, session_id: &str) -> Result<Vec<Recording>>;

    /// Gravações que ficaram `recording`/`paused` (órfãs de um crash).
    fn list_unfinished(&self) -> Result<Vec<Recording>>;
}
