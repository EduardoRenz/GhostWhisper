//! Repositório SQLite do diário de perguntas (aba "Sessão", BR-052).
//!
//! Usa as tabelas `journal_entries`/`journal_images` e o FTS5 `journal_fts` do
//! schema. O FTS é casado pela **chave de negócio** (`id`), não pelo `rowid`:
//! o `rowid` implícito é renumerado num `VACUUM` e o casamento passaria a
//! apontar para a linha errada, em silêncio (ISS-041).

use rusqlite::{params, Row};

use crate::domain::journal::{
    JournalEntry, JournalImage, JournalRequestSource, JournalStatus,
};
use crate::error::Result;
use crate::ports::journal_repository::JournalRepository;

use super::connection::{lock, SharedConnection};
use super::fts::escape_query;

const COLUMNS: &str = "id, session_id, question, answer, asked_at_ms, answered_at_ms, \
                       status, request_source, provider, model, error";

/// Implementa `JournalRepository` em SQLite.
pub struct LocalJournalRepository {
    conn: SharedConnection,
}

impl LocalJournalRepository {
    /// Cria o repositório sobre a conexão compartilhada.
    pub fn new(conn: SharedConnection) -> Self {
        Self { conn }
    }
}

impl JournalRepository for LocalJournalRepository {
    fn insert_question(&self, entry: &JournalEntry) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "INSERT INTO journal_entries \
             (id, session_id, question, answer, asked_at_ms, answered_at_ms, \
              status, request_source, provider, model, error) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                entry.id,
                entry.session_id,
                entry.question,
                entry.answer,
                entry.asked_at_ms,
                entry.answered_at_ms,
                entry.status.as_str(),
                entry.request_source.as_str(),
                entry.provider,
                entry.model,
                entry.error,
            ],
        )?;
        Ok(())
    }

    fn complete_answer(
        &self,
        id: &str,
        answer: &str,
        answered_at_ms: i64,
        provider: Option<String>,
        model: Option<String>,
    ) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "UPDATE journal_entries \
             SET answer = ?2, answered_at_ms = ?3, status = 'answered', \
                 provider = COALESCE(?4, provider), model = COALESCE(?5, model) \
             WHERE id = ?1",
            params![id, answer, answered_at_ms, provider, model],
        )?;
        Ok(())
    }

    fn fail(&self, id: &str, status: JournalStatus, error: Option<String>) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "UPDATE journal_entries SET status = ?2, error = ?3 WHERE id = ?1",
            params![id, status.as_str(), error],
        )?;
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<JournalEntry>> {
        let conn = lock(&self.conn)?;
        let query = format!("SELECT {COLUMNS} FROM journal_entries WHERE id = ?1");
        let mut stmt = conn.prepare(&query)?;
        let mut rows = stmt.query_map(params![id], map_entry)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    fn list_by_session(&self, session_id: &str) -> Result<Vec<JournalEntry>> {
        let conn = lock(&self.conn)?;
        let query = format!(
            "SELECT {COLUMNS} FROM journal_entries WHERE session_id = ?1 \
             ORDER BY asked_at_ms ASC, id ASC"
        );
        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![session_id], map_entry)?;
        collect(rows)
    }

    fn search(&self, session_id: &str, query: &str, limit: usize) -> Result<Vec<JournalEntry>> {
        let Some(fts_query) = escape_query(query) else {
            return Ok(Vec::new());
        };
        let conn = lock(&self.conn)?;
        // O FTS é filtrado pela chave de negócio (`entry_id`), não pelo rowid
        // (ISS-041): o rowid implícito é renumerado num `VACUUM`.
        let sql = format!(
            "SELECT {COLUMNS} FROM journal_entries j \
             WHERE j.session_id = ?1 \
               AND j.id IN (SELECT entry_id FROM journal_fts WHERE journal_fts MATCH ?2) \
             ORDER BY j.asked_at_ms ASC, id ASC LIMIT ?3"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![session_id, fts_query, limit as i64], map_entry)?;
        collect(rows)
    }

    fn attach_image(&self, image: &JournalImage) -> Result<()> {
        let conn = lock(&self.conn)?;
        conn.execute(
            "INSERT INTO journal_images (id, entry_id, mime, data, width, height) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                image.id,
                image.entry_id,
                image.mime,
                image.data,
                image.width.map(|value| value as i64),
                image.height.map(|value| value as i64),
            ],
        )?;
        Ok(())
    }

    fn images_of(&self, entry_id: &str) -> Result<Vec<JournalImage>> {
        let conn = lock(&self.conn)?;
        let query = "SELECT id, entry_id, mime, data, width, height FROM journal_images \
                     WHERE entry_id = ?1 ORDER BY id ASC";
        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map(params![entry_id], map_image)?;
        let mut images = Vec::new();
        for row in rows {
            images.push(row?);
        }
        Ok(images)
    }

    fn delete(&self, id: &str) -> Result<()> {
        let conn = lock(&self.conn)?;
        // As imagens primeiro: o `ON DELETE CASCADE` cobre o caso normal, mas
        // não queremos depender do pragma de chaves estrangeiras estar ligado.
        conn.execute("DELETE FROM journal_images WHERE entry_id = ?1", params![id])?;
        conn.execute("DELETE FROM journal_entries WHERE id = ?1", params![id])?;
        Ok(())
    }
}

fn collect(
    rows: rusqlite::MappedRows<'_, impl FnMut(&Row<'_>) -> rusqlite::Result<JournalEntry>>,
) -> Result<Vec<JournalEntry>> {
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

fn map_entry(row: &Row<'_>) -> rusqlite::Result<JournalEntry> {
    let status: String = row.get(6)?;
    let source: String = row.get(7)?;
    Ok(JournalEntry {
        id: row.get(0)?,
        session_id: row.get(1)?,
        question: row.get(2)?,
        answer: row.get(3)?,
        asked_at_ms: row.get(4)?,
        answered_at_ms: row.get(5)?,
        status: JournalStatus::from_str(&status).unwrap_or(JournalStatus::Pending),
        request_source: JournalRequestSource::from_str(&source)
            .unwrap_or(JournalRequestSource::Search),
        provider: row.get(8)?,
        model: row.get(9)?,
        error: row.get(10)?,
    })
}

fn map_image(row: &Row<'_>) -> rusqlite::Result<JournalImage> {
    let width: Option<i64> = row.get(4)?;
    let height: Option<i64> = row.get(5)?;
    Ok(JournalImage {
        id: row.get(0)?,
        entry_id: row.get(1)?,
        mime: row.get(2)?,
        data: row.get(3)?,
        width: width.map(|value| value as u32),
        height: height.map(|value| value as u32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::sqlite::local_session_repository::{
        tests_support, LocalSessionRepository,
    };
    use crate::ports::repositories::SessionRepository;

    fn repo() -> (LocalJournalRepository, SharedConnection) {
        let conn = tests_support::in_memory();
        let sessions = LocalSessionRepository::new(conn.clone());
        sessions.create(&tests_support::session("s1", 0)).unwrap();
        sessions.create(&tests_support::session("s2", 0)).unwrap();
        (LocalJournalRepository::new(conn.clone()), conn)
    }

    fn entry(id: &str, session: &str, question: &str, at: i64) -> JournalEntry {
        JournalEntry {
            id: id.into(),
            session_id: session.into(),
            question: question.into(),
            answer: None,
            asked_at_ms: at,
            answered_at_ms: None,
            status: JournalStatus::Pending,
            request_source: JournalRequestSource::Search,
            provider: None,
            model: None,
            error: None,
        }
    }

    #[test]
    fn inserts_lists_and_answers() {
        let (repo, _conn) = repo();
        repo.insert_question(&entry("e2", "s1", "segunda", 1_000)).unwrap();
        repo.insert_question(&entry("e1", "s1", "primeira", 0)).unwrap();
        repo.insert_question(&entry("e3", "s2", "outra sessão", 0)).unwrap();

        let listed = repo.list_by_session("s1").expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].question, "primeira");
        assert_eq!(listed[1].question, "segunda");
        assert_eq!(listed[0].status, JournalStatus::Pending);

        repo.complete_answer("e1", "a resposta", 500, Some("openai".into()), Some("gpt".into()))
            .unwrap();

        let answered = repo.get("e1").expect("get").expect("presente");
        assert_eq!(answered.answer.as_deref(), Some("a resposta"));
        assert_eq!(answered.answered_at_ms, Some(500));
        assert_eq!(answered.status, JournalStatus::Answered);
        assert_eq!(answered.provider.as_deref(), Some("openai"));
    }

    #[test]
    fn marks_a_failure() {
        let (repo, _conn) = repo();
        repo.insert_question(&entry("e1", "s1", "pergunta", 0)).unwrap();
        repo.fail("e1", JournalStatus::Error, Some("deu ruim".into()))
            .unwrap();

        let failed = repo.get("e1").expect("get").expect("presente");
        assert_eq!(failed.status, JournalStatus::Error);
        assert_eq!(failed.error.as_deref(), Some("deu ruim"));
    }

    #[test]
    fn searches_by_question_and_by_answer() {
        let (repo, _conn) = repo();
        repo.insert_question(&entry("e1", "s1", "como inverte linked list", 0))
            .unwrap();
        repo.insert_question(&entry("e2", "s1", "outra pergunta", 1_000)).unwrap();
        repo.complete_answer("e2", "a resposta fala de linked list", 1_500, None, None)
            .unwrap();

        let found = repo.search("s1", "linked", 10).expect("search");
        assert_eq!(found.len(), 2, "a pergunta e a resposta casam");

        let found = repo.search("s1", "linked list", 10).expect("search");
        assert_eq!(found.len(), 2, "os dois termos casam");

        let found = repo.search("s1", "ausente", 10).expect("search");
        assert!(found.is_empty());
    }

    #[test]
    fn the_search_is_limited_to_the_session() {
        let (repo, _conn) = repo();
        repo.insert_question(&entry("e1", "s1", "palavra comum", 0)).unwrap();
        repo.insert_question(&entry("e2", "s2", "palavra comum", 0)).unwrap();

        let found = repo.search("s1", "comum", 10).expect("search");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "e1");
    }

    #[test]
    fn fts_syntax_from_the_user_does_not_break_the_search() {
        let (repo, _conn) = repo();
        repo.insert_question(&entry("e1", "s1", "aspas \" aqui", 0)).unwrap();
        repo.insert_question(&entry("e2", "s1", "asterisco * aqui", 0)).unwrap();

        // Sem escape, qualquer um destes derrubaria a consulta inteira e a
        // busca voltaria vazia, sem aviso (ISS-042).
        for query in ["\"", "a*b", "x:y", "NEAR(", "-", "AND OR"] {
            assert!(
                repo.search("s1", query, 10).is_ok(),
                "a query {query:?} não pode derrubar a busca"
            );
        }

        // E a busca normal continua funcionando.
        let found = repo.search("s1", "asterisco", 10).expect("search");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "e2");
    }

    #[test]
    fn an_empty_search_returns_nothing() {
        let (repo, _conn) = repo();
        repo.insert_question(&entry("e1", "s1", "qualquer", 0)).unwrap();
        assert!(repo.search("s1", "   ", 10).expect("search").is_empty());
    }

    #[test]
    fn attaches_and_lists_images() {
        let (repo, _conn) = repo();
        repo.insert_question(&entry("e1", "s1", "pergunta", 0)).unwrap();
        repo.attach_image(&JournalImage {
            id: "i1".into(),
            entry_id: "e1".into(),
            mime: "image/jpeg".into(),
            data: vec![1, 2, 3],
            width: Some(1280),
            height: Some(720),
        })
        .unwrap();

        let images = repo.images_of("e1").expect("images");
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].mime, "image/jpeg");
        assert_eq!(images[0].data, vec![1, 2, 3]);
        assert_eq!(images[0].width, Some(1280));

        assert!(repo.images_of("outra").expect("images").is_empty());
    }

    #[test]
    fn deleting_an_entry_deletes_its_images() {
        let (repo, _conn) = repo();
        repo.insert_question(&entry("e1", "s1", "pergunta", 0)).unwrap();
        repo.attach_image(&JournalImage {
            id: "i1".into(),
            entry_id: "e1".into(),
            mime: "image/jpeg".into(),
            data: vec![1],
            width: None,
            height: None,
        })
        .unwrap();

        repo.delete("e1").expect("delete");

        assert!(repo.get("e1").expect("get").is_none());
        assert!(repo.images_of("e1").expect("images").is_empty());
    }
}
