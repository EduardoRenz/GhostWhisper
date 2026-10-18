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
    Ok(())
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
