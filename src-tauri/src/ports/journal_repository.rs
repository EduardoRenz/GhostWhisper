//! Porta de persistência do diário de perguntas (aba "Sessão", BR-052).
//!
//! Adaptador: `LocalJournalRepository`; dobra: `InMemoryJournalRepository`.

use crate::domain::journal::{JournalEntry, JournalImage, JournalStatus};
use crate::error::Result;

/// Persistência do diário de perguntas de uma sessão.
pub trait JournalRepository: Send + Sync {
    /// Registra a pergunta.
    ///
    /// A pergunta é persistida **antes** da chamada à IA: um crash no meio
    /// deixa a entrada no diário como `pending` (RNF-06).
    fn insert_question(&self, entry: &JournalEntry) -> Result<()>;

    /// Grava a resposta e marca a entrada como respondida.
    fn complete_answer(
        &self,
        id: &str,
        answer: &str,
        answered_at_ms: i64,
        provider: Option<String>,
        model: Option<String>,
    ) -> Result<()>;

    /// Marca a entrada como erro ou cancelada.
    fn fail(&self, id: &str, status: JournalStatus, error: Option<String>) -> Result<()>;

    /// Uma entrada pelo id.
    fn get(&self, id: &str) -> Result<Option<JournalEntry>>;

    /// Diário da sessão, em ordem cronológica (mais antiga primeiro).
    fn list_by_session(&self, session_id: &str) -> Result<Vec<JournalEntry>>;

    /// Busca full-text (pergunta e resposta) dentro da sessão.
    fn search(&self, session_id: &str, query: &str, limit: usize) -> Result<Vec<JournalEntry>>;

    /// Anexa uma imagem a uma entrada.
    fn attach_image(&self, image: &JournalImage) -> Result<()>;

    /// Imagens de uma entrada.
    fn images_of(&self, entry_id: &str) -> Result<Vec<JournalImage>>;

    /// Remove uma entrada (e as imagens dela, em cascade).
    fn delete(&self, id: &str) -> Result<()>;
}
