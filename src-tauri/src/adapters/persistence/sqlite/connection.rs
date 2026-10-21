//! Conexão SQLite e inicialização do schema.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::{GhostWhisperError, Result};

const SCHEMA: &str = include_str!("../schema.sql");

/// Conexão compartilhada entre repositórios.
pub type SharedConnection = Arc<Mutex<Connection>>;

/// Abre (ou cria) um banco em `path` e aplica o schema.
pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    initialize(&conn)?;
    Ok(conn)
}

/// Abre um banco em memória (usado pelos testes).
#[cfg(test)]
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    initialize(&conn)?;
    Ok(conn)
}

/// Aplica o schema (idempotente).
pub fn initialize(conn: &Connection) -> Result<()> {
    // Chaves estrangeiras ligadas: o schema depende delas para o `ON DELETE
    // CASCADE` (sessão → transcrições/mensagens). Sem o pragma, o SQLite aceita
    // (e ignora) a FK — e uma sessão excluída deixaria órfãos.
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(SCHEMA)?;
    migrate_columns(conn)?;
    Ok(())
}

/// Migrações de **coluna** (o `schema.sql` só cria tabelas/índices que faltam).
///
/// `CREATE TABLE IF NOT EXISTS` não altera uma tabela existente: colunas novas
/// entram por `ALTER TABLE`, uma vez, quando ainda não existem.
fn migrate_columns(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "journal_entries", "summary", "TEXT")?;
    // v4: `recordings.session_id` deixa de ser UNIQUE — continuar uma sessão
    // antiga inicia uma **nova** gravação (BR-065). Um banco da v3 ainda tem o
    // índice único; a tabela é reconstruída uma vez.
    rebuild_recordings_without_unique(conn)?;
    Ok(())
}

/// A tabela `recordings` ainda tem o índice único de `session_id`?
fn has_unique_index_on(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA index_list('{table}')"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let unique: i64 = row.get(2)?;
        if unique != 1 {
            continue;
        }
        let name: String = row.get(1)?;
        let mut info = conn.prepare(&format!("PRAGMA index_info('{name}')"))?;
        let mut columns = info.query([])?;
        let mut names: Vec<String> = Vec::new();
        while let Some(entry) = columns.next()? {
            names.push(entry.get::<_, String>(2)?);
        }
        if names == [column.to_string()] {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Reconstrói `recordings` sem o `UNIQUE(session_id)` (migração da v4).
fn rebuild_recordings_without_unique(conn: &Connection) -> Result<()> {
    if !has_unique_index_on(conn, "recordings", "session_id")? {
        return Ok(());
    }

    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
    let migration = conn.execute_batch(
        "BEGIN;
         ALTER TABLE recordings RENAME TO recordings_old;
         CREATE TABLE recordings (
           id             TEXT PRIMARY KEY,
           session_id     TEXT NOT NULL,
           kind           TEXT NOT NULL CHECK(kind IN ('video', 'audio')),
           status         TEXT NOT NULL
             CHECK(status IN ('recording', 'paused', 'finalizing', 'ready', 'failed',
                              'discarded', 'interrupted')),
           path           TEXT,
           segments_dir   TEXT,
           size_bytes     INTEGER NOT NULL DEFAULT 0,
           duration_ms    INTEGER NOT NULL DEFAULT 0,
           width          INTEGER NOT NULL DEFAULT 0,
           height         INTEGER NOT NULL DEFAULT 0,
           fps            INTEGER NOT NULL DEFAULT 0,
           started_at_ms  INTEGER NOT NULL,
           finished_at_ms INTEGER,
           error          TEXT,
           created_at     INTEGER DEFAULT (strftime('%s', 'now')),
           FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
         );
         INSERT INTO recordings
           (id, session_id, kind, status, path, segments_dir, size_bytes, duration_ms,
            width, height, fps, started_at_ms, finished_at_ms, error, created_at)
           SELECT
             id, session_id, kind, status, path, segments_dir, size_bytes, duration_ms,
             width, height, fps, started_at_ms, finished_at_ms, error, created_at
           FROM recordings_old;
         DROP TABLE recordings_old;
         CREATE INDEX IF NOT EXISTS idx_recordings_status ON recordings(status);
         CREATE INDEX IF NOT EXISTS idx_recordings_session
           ON recordings(session_id, started_at_ms);
         COMMIT;",
    );
    if migration.is_err() {
        let _ = conn.execute_batch("ROLLBACK;");
    }
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    migration?;
    tracing::info!("tabela recordings reconstruída (várias gravações por sessão)");
    Ok(())
}

/// Adiciona a coluna quando ela ainda não existe (idempotente).
fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    kind: &str,
) -> Result<()> {
    if has_column(conn, table, column)? {
        return Ok(());
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {kind}"),
        [],
    )?;
    tracing::info!(table, column, "coluna adicionada");
    Ok(())
}

/// A tabela já tem a coluna? (`PRAGMA table_info`)
fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Envolve uma conexão em `Arc<Mutex<..>>` para compartilhar entre adaptadores.
pub fn shared(conn: Connection) -> SharedConnection {
    Arc::new(Mutex::new(conn))
}

/// Obtém o lock da conexão, mapeando envenenamento para erro de domínio.
pub fn lock(conn: &SharedConnection) -> Result<MutexGuard<'_, Connection>> {
    conn.lock()
        .map_err(|_| GhostWhisperError::Other("lock do banco envenenado".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Banco antigo: a coluna `summary` do diário entra pela migração (BR-053).
    #[test]
    fn an_old_database_gains_the_journal_summary_column() {
        let conn = Connection::open_in_memory().expect("in-memory");
        // A tabela como era antes do resumo (com as colunas que os índices e
        // os triggers do schema usam).
        conn.execute_batch(
            "CREATE TABLE journal_entries (
               rowid_pk INTEGER PRIMARY KEY AUTOINCREMENT,
               id TEXT NOT NULL UNIQUE,
               session_id TEXT NOT NULL,
               question TEXT NOT NULL,
               answer TEXT,
               asked_at_ms INTEGER NOT NULL,
               answered_at_ms INTEGER,
               status TEXT NOT NULL DEFAULT 'pending',
               request_source TEXT NOT NULL DEFAULT 'search',
               provider TEXT,
               model TEXT,
               error TEXT,
               created_at INTEGER
             );",
        )
        .expect("schema antigo");
        assert!(!has_column(&conn, "journal_entries", "summary").expect("antes"));

        initialize(&conn).expect("migra");

        assert!(has_column(&conn, "journal_entries", "summary").expect("depois"));
        // Idempotente: o boot seguinte não tenta de novo.
        initialize(&conn).expect("segunda vez");
    }

    /// Banco da v3: `recordings.session_id` era `UNIQUE`; a v4 permite várias
    /// gravações por sessão (continuar uma sessão antiga cria outra).
    #[test]
    fn an_old_recordings_table_loses_the_unique_session_index() {
        let conn = Connection::open_in_memory().expect("in-memory");
        conn.execute_batch(
            "CREATE TABLE sessions (
               id TEXT PRIMARY KEY,
               started_at INTEGER NOT NULL,
               status TEXT
             );
             CREATE TABLE recordings (
               id TEXT PRIMARY KEY,
               session_id TEXT NOT NULL UNIQUE,
               kind TEXT NOT NULL,
               status TEXT NOT NULL,
               path TEXT,
               segments_dir TEXT,
               size_bytes INTEGER NOT NULL DEFAULT 0,
               duration_ms INTEGER NOT NULL DEFAULT 0,
               width INTEGER NOT NULL DEFAULT 0,
               height INTEGER NOT NULL DEFAULT 0,
               fps INTEGER NOT NULL DEFAULT 0,
               started_at_ms INTEGER NOT NULL,
               finished_at_ms INTEGER,
               error TEXT,
               created_at INTEGER
             );
             INSERT INTO sessions (id, started_at, status) VALUES ('s1', 0, 'ended');
             INSERT INTO recordings (id, session_id, kind, status, started_at_ms)
               VALUES ('r1', 's1', 'video', 'ready', 1);",
        )
        .expect("schema v3");
        assert!(
            has_unique_index_on(&conn, "recordings", "session_id").expect("antes"),
            "o schema antigo tinha UNIQUE"
        );

        initialize(&conn).expect("migra");

        assert!(!has_unique_index_on(&conn, "recordings", "session_id").expect("depois"));
        conn.execute(
            "INSERT INTO recordings (id, session_id, kind, status, started_at_ms) \
             VALUES ('r2', 's1', 'video', 'ready', 2)",
            [],
        )
        .expect("segunda gravação na mesma sessão");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM recordings", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 2);
        // Idempotente: o boot seguinte não reconstrói de novo.
        initialize(&conn).expect("segunda vez");
    }

    /// O conjunto de índices do schema (BR-058).
    ///
    /// Fixa o que **existe**: os compostos (listagem ordenada sem *temp
    /// b-tree*), o `status` (usado por `end_all_active`) e a ausência dos
    /// índices simples (cobertos) e do `user_id` (morto: `user_id` é sempre
    /// `NULL` — BR-002).
    #[test]
    fn the_schema_has_the_expected_indexes() {
        let conn = open_in_memory().expect("in-memory");
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND sql IS NOT NULL")
            .expect("prepara");
        let names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("consulta")
            .filter_map(|row| row.ok())
            .collect();

        for expected in [
            "idx_messages_session_time",
            "idx_transcriptions_session_time",
            "idx_sessions_started_at",
            "idx_sessions_status",
            "idx_journal_session_time",
            "idx_journal_images_entry",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "falta o índice {expected} (tem {names:?})"
            );
        }

        for removed in [
            "idx_messages_session",
            "idx_messages_timestamp",
            "idx_transcriptions_session",
            "idx_transcriptions_timestamp",
            "idx_sessions_user",
        ] {
            assert!(
                !names.iter().any(|name| name == removed),
                "o índice {removed} deveria ter sido removido"
            );
        }
    }

    /// A listagem ordenada por sessão **não** paga *temp b-tree* (BR-058).
    #[test]
    fn the_ordered_listing_uses_the_composite_index() {
        let conn = open_in_memory().expect("in-memory");
        let plan: String = conn
            .query_row(
                "EXPLAIN QUERY PLAN SELECT id FROM transcriptions WHERE session_id = ?1 ORDER BY timestamp ASC",
                ["s1"],
                // As colunas do plano são: id, parent, notused, detail.
                |row| row.get(3),
            )
            .expect("plano");
        assert!(
            plan.contains("idx_transcriptions_session_time"),
            "a listagem deveria usar o índice composto: {plan}"
        );
        assert!(
            !plan.contains("TEMP B-TREE"),
            "a listagem não deveria ordenar em memória: {plan}"
        );
    }

    #[test]
    fn initialize_creates_tables_and_fts() {
        let conn = open_in_memory().expect("in-memory");
        // As tabelas principais existem.
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name IN ('sessions','messages','transcriptions','transcriptions_fts','meeting_summaries')",
                [],
                |row| row.get(0),
            )
            .expect("consulta");
        assert_eq!(count, 5);
    }
}
