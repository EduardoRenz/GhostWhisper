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
