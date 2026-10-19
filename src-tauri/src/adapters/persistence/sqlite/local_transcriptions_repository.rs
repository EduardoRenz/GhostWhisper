//! Repositório SQLite de transcrições (com busca FTS5).

use rusqlite::{params, Row};

use crate::domain::transcription::{Speaker, TranscriptSegment};
use crate::error::Result;
use crate::ports::repositories::TranscriptionsRepository;

use super::connection::{lock, SharedConnection};

const COLUMNS: &str = "id, session_id, speaker, text, timestamp, end_ms, is_final";

/// Implementa `TranscriptionsRepository` em SQLite.
pub struct LocalTranscriptionsRepository {
    conn: SharedConnection,
}

impl LocalTranscriptionsRepository {
    /// Cria o repositório sobre a conexão compartilhada.
    pub fn new(conn: SharedConnection) -> Self {
        Self { conn }
    }
}

impl TranscriptionsRepository for LocalTranscriptionsRepository {
    fn insert(&self, segment: &TranscriptSegment) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "INSERT INTO transcriptions (id, session_id, speaker, text, timestamp, end_ms, is_final) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                segment.id,
                segment.session_id,
                segment.speaker.as_str(),
                segment.text,
                segment.start_ms,
                segment.end_ms,
                segment.is_final as i64,
            ],
        )?;
        Ok(())
    }

    fn list_by_session(&self, session_id: &str) -> Result<Vec<TranscriptSegment>> {
        let conn = lock(&self.conn)?;
        let query = format!(
            "SELECT {COLUMNS} FROM transcriptions WHERE session_id = ?1 ORDER BY timestamp ASC"
        );
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![session_id], map_segment)?;
        collect(rows)
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<TranscriptSegment>> {
        let conn = lock(&self.conn)?;
        let sql = format!(
            "SELECT {COLUMNS} FROM transcriptions t \
             WHERE t.rowid IN (SELECT rowid FROM transcriptions_fts WHERE transcriptions_fts MATCH ?1) \
             ORDER BY t.timestamp ASC LIMIT ?2"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![query, limit as i64], map_segment)?;
        collect(rows)
    }

    fn latest_final_ms(&self, session_id: &str) -> Result<Option<i64>> {
        let conn = lock(&self.conn)?;
        let value = conn.query_row(
            "SELECT MAX(timestamp) FROM transcriptions \
             WHERE session_id = ?1 AND is_final = 1 AND TRIM(text) <> ''",
            params![session_id],
            |row| row.get::<_, Option<i64>>(0),
        )?;
        Ok(value)
    }
}

fn collect(
    rows: rusqlite::MappedRows<'_, impl FnMut(&Row<'_>) -> rusqlite::Result<TranscriptSegment>>,
) -> Result<Vec<TranscriptSegment>> {
    let mut segments = Vec::new();
    for row in rows {
        segments.push(row?);
    }
    Ok(segments)
}

fn map_segment(row: &Row<'_>) -> rusqlite::Result<TranscriptSegment> {
    let speaker: String = row.get(2)?;
    Ok(TranscriptSegment {
        id: row.get(0)?,
        session_id: row.get(1)?,
        speaker: Speaker::from_str(&speaker).unwrap_or(Speaker::Me),
        text: row.get(3)?,
        start_ms: row.get(4)?,
        end_ms: row.get(5)?,
        is_final: row.get::<_, i64>(6)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::sqlite::local_session_repository::{
        tests_support, LocalSessionRepository,
    };
    use crate::ports::repositories::SessionRepository;

    fn repo() -> (LocalTranscriptionsRepository, SharedConnection) {
        let conn = tests_support::in_memory();
        // Sessões são pré-requisito das transcrições (FK).
        let sessions = LocalSessionRepository::new(conn.clone());
        sessions.create(&tests_support::session("s1", 0)).unwrap();
        sessions.create(&tests_support::session("s2", 0)).unwrap();
        (LocalTranscriptionsRepository::new(conn.clone()), conn)
    }

    fn segment(id: &str, session_id: &str, text: &str, start_ms: i64) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            session_id: session_id.to_string(),
            speaker: Speaker::Me,
            text: text.to_string(),
            start_ms,
            end_ms: start_ms + 500,
            is_final: true,
        }
    }

    #[test]
    fn insert_list_and_search() {
        let (repo, _conn) = repo();
        repo.insert(&segment("1", "s1", "olá mundo", 0)).unwrap();
        repo.insert(&segment("2", "s1", "segunda frase", 1000))
            .unwrap();
        repo.insert(&segment("3", "s2", "outra sessão", 0)).unwrap();

        let s1 = repo.list_by_session("s1").expect("list");
        assert_eq!(s1.len(), 2);
        assert_eq!(s1[0].text, "olá mundo");
        assert_eq!(s1[1].start_ms, 1000);

        let found = repo.search("mundo", 10).expect("search");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "1");

        let found = repo.search("sessão", 10).expect("search 2");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "3");

        // Cursor barato do agendador.
        assert_eq!(repo.latest_final_ms("s1").expect("latest"), Some(1_000));
        assert_eq!(repo.latest_final_ms("s3").expect("latest"), None);
    }
}
