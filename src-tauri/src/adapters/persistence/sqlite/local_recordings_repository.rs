//! Repositório SQLite da gravação da sessão (BR-065/BR-067).
//!
//! Uma linha por sessão; o arquivo e os segmentos vivem no disco e são
//! apagados pelo serviço (o banco guarda só os metadados).

use rusqlite::{params, Row};

use crate::domain::recording::{Recording, RecordingKind, RecordingStatus};
use crate::error::Result;
use crate::ports::recordings_repository::RecordingsRepository;

use super::connection::{lock, SharedConnection};

const COLUMNS: &str = "id, session_id, kind, status, path, segments_dir, size_bytes, \
                       duration_ms, width, height, fps, started_at_ms, finished_at_ms, error";

/// Implementa `RecordingsRepository` em SQLite.
pub struct LocalRecordingsRepository {
    conn: SharedConnection,
}

impl LocalRecordingsRepository {
    /// Cria o repositório sobre a conexão compartilhada.
    pub fn new(conn: SharedConnection) -> Self {
        Self { conn }
    }
}

impl RecordingsRepository for LocalRecordingsRepository {
    fn upsert(&self, recording: &Recording) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "INSERT INTO recordings \
             (id, session_id, kind, status, path, segments_dir, size_bytes, duration_ms, \
              width, height, fps, started_at_ms, finished_at_ms, error) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14) \
             ON CONFLICT(id) DO UPDATE SET \
               kind = excluded.kind, status = excluded.status, path = excluded.path, \
               segments_dir = excluded.segments_dir, size_bytes = excluded.size_bytes, \
               duration_ms = excluded.duration_ms, width = excluded.width, \
               height = excluded.height, fps = excluded.fps, \
               finished_at_ms = excluded.finished_at_ms, error = excluded.error",
            params![
                recording.id,
                recording.session_id,
                recording.kind.as_str(),
                recording.status.as_str(),
                recording.path,
                recording.segments_dir,
                recording.size_bytes as i64,
                recording.duration_ms,
                recording.width as i64,
                recording.height as i64,
                recording.fps as i64,
                recording.started_at_ms,
                recording.finished_at_ms,
                recording.error,
            ],
        )?;
        Ok(())
    }

    fn update_status(&self, id: &str, status: RecordingStatus, error: Option<&str>) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "UPDATE recordings SET status = ?2, error = ?3 WHERE id = ?1",
            params![id, status.as_str(), error],
        )?;
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
        let conn = lock(&self.conn)?;
        conn.execute(
            "UPDATE recordings \
             SET path = ?2, size_bytes = ?3, duration_ms = ?4, finished_at_ms = ?5, \
                 status = 'ready', error = NULL \
             WHERE id = ?1",
            params![id, path, size_bytes as i64, duration_ms, finished_at_ms],
        )?;
        Ok(())
    }

    fn get_by_session(&self, session_id: &str) -> Result<Option<Recording>> {
        let conn = lock(&self.conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM recordings WHERE session_id = ?1 \
             ORDER BY started_at_ms DESC, created_at DESC LIMIT 1"
        ))?;
        let mut rows = stmt.query(params![session_id])?;
        Ok(rows.next()?.map(map_row).transpose()?)
    }

    fn list_by_session(&self, session_id: &str) -> Result<Vec<Recording>> {
        let conn = lock(&self.conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM recordings WHERE session_id = ?1 \
             ORDER BY started_at_ms"
        ))?;
        let rows = stmt.query_map(params![session_id], map_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn list_unfinished(&self) -> Result<Vec<Recording>> {
        let conn = lock(&self.conn)?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {COLUMNS} FROM recordings \
             WHERE status IN ('recording', 'paused') ORDER BY started_at_ms"
        ))?;
        let rows = stmt.query_map([], map_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn map_row(row: &Row<'_>) -> rusqlite::Result<Recording> {
    let kind: String = row.get(2)?;
    let status: String = row.get(3)?;
    Ok(Recording {
        id: row.get(0)?,
        session_id: row.get(1)?,
        kind: RecordingKind::from_str(&kind).unwrap_or(RecordingKind::Video),
        status: RecordingStatus::from_str(&status).unwrap_or(RecordingStatus::Interrupted),
        path: row.get(4)?,
        segments_dir: row.get(5)?,
        size_bytes: row.get::<_, i64>(6)?.max(0) as u64,
        duration_ms: row.get(7)?,
        width: row.get::<_, i64>(8)?.max(0) as u32,
        height: row.get::<_, i64>(9)?.max(0) as u32,
        fps: row.get::<_, i64>(10)?.max(0) as u32,
        started_at_ms: row.get(11)?,
        finished_at_ms: row.get(12)?,
        error: row.get(13)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::sqlite::connection::open_in_memory;
    use crate::domain::recording::{RecordingKind, RecordingStatus};

    fn repository() -> LocalRecordingsRepository {
        let conn = open_in_memory().expect("banco");
        LocalRecordingsRepository::new(crate::adapters::persistence::sqlite::connection::shared(
            conn,
        ))
    }

    fn recording(session_id: &str) -> Recording {
        Recording {
            id: format!("rec-{session_id}"),
            session_id: session_id.to_string(),
            kind: RecordingKind::Video,
            status: RecordingStatus::Recording,
            path: None,
            segments_dir: Some("/tmp/rec".to_string()),
            size_bytes: 0,
            duration_ms: 0,
            width: 1920,
            height: 1080,
            fps: 8,
            started_at_ms: 1_000,
            finished_at_ms: None,
            error: None,
        }
    }

    fn session(repo: &LocalRecordingsRepository, id: &str) {
        let conn = lock(repo.conn()).expect("lock");
        conn.execute(
            "INSERT INTO sessions (id, started_at, provider, status) VALUES (?1, 0, 'local', 'active')",
            params![id],
        )
        .expect("sessão");
    }

    impl LocalRecordingsRepository {
        fn conn(&self) -> &SharedConnection {
            &self.conn
        }
    }

    #[test]
    fn round_trips_status_and_artifact() {
        let repo = repository();
        session(&repo, "s1");
        repo.upsert(&recording("s1")).expect("upsert");

        let stored = repo.get_by_session("s1").expect("get").expect("existe");
        assert_eq!(stored.status, RecordingStatus::Recording);
        assert_eq!(stored.fps, 8);

        repo.update_status("rec-s1", RecordingStatus::Paused, None)
            .expect("pause");
        assert_eq!(
            repo.get_by_session("s1")
                .expect("get")
                .expect("existe")
                .status,
            RecordingStatus::Paused
        );

        repo.set_artifact("rec-s1", "/tmp/rec/s1.mp4", 1234, 5000, 9_000)
            .expect("artifact");
        let stored = repo.get_by_session("s1").expect("get").expect("existe");
        assert_eq!(stored.status, RecordingStatus::Ready);
        assert_eq!(stored.path.as_deref(), Some("/tmp/rec/s1.mp4"));
        assert_eq!(stored.size_bytes, 1234);
        assert_eq!(stored.finished_at_ms, Some(9_000));
    }

    #[test]
    fn keeps_multiple_recordings_per_session_and_returns_the_latest() {
        let repo = repository();
        session(&repo, "s1");

        let mut first = recording("s1");
        first.id = "r1".to_string();
        first.started_at_ms = 1;
        let mut second = recording("s1");
        second.id = "r2".to_string();
        second.started_at_ms = 2;

        repo.upsert(&first).expect("primeira");
        repo.upsert(&second).expect("segunda");

        assert_eq!(
            repo.get_by_session("s1").expect("get").expect("existe").id,
            "r2",
            "o modal mostra a execução mais recente"
        );
        assert_eq!(repo.list_by_session("s1").expect("lista").len(), 2);
    }

    #[test]
    fn lists_only_unfinished_recordings() {
        let repo = repository();
        session(&repo, "s1");
        session(&repo, "s2");
        repo.upsert(&recording("s1")).expect("s1");
        repo.upsert(&recording("s2")).expect("s2");
        repo.update_status("rec-s2", RecordingStatus::Ready, None)
            .expect("ready");
        let unfinished = repo.list_unfinished().expect("lista");
        assert_eq!(unfinished.len(), 1);
        assert_eq!(unfinished[0].session_id, "s1");
    }

    #[test]
    fn deleting_the_session_cascades() {
        let repo = repository();
        session(&repo, "s1");
        repo.upsert(&recording("s1")).expect("upsert");
        let conn = lock(repo.conn()).expect("lock");
        conn.execute("DELETE FROM sessions WHERE id = 's1'", [])
            .expect("delete");
        drop(conn);
        assert!(repo.get_by_session("s1").expect("get").is_none());
    }
}
