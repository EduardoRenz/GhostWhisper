//! Portas de persistência (sessões, transcrições e mensagens).

use crate::domain::message::ConversationMessage;
use crate::domain::session::Session;
use crate::domain::transcription::TranscriptSegment;
use crate::error::Result;

/// Persistência de sessões.
///
/// Adaptador: `LocalSessionRepository`; dobra: `InMemorySessionRepository`.
pub trait SessionRepository: Send + Sync {
    fn create(&self, session: &Session) -> Result<()>;
    fn end(&self, id: &str, ended_at_ms: i64) -> Result<()>;
    fn get(&self, id: &str) -> Result<Option<Session>>;
    fn list_recent(&self, limit: usize) -> Result<Vec<Session>>;
    /// Reativa uma sessão encerrada (`status = active`, `ended_at = NULL`).
    fn resume(&self, id: &str) -> Result<()>;
    /// Encerra todas as sessões ainda `active` e retorna quantas foram.
    ///
    /// Usado no boot: se o app foi fechado durante uma gravação, a sessão fica
    /// órfã em `active` (BR-019).
    fn end_all_active(&self, ended_at_ms: i64) -> Result<usize>;
    /// Define o título (`subject`) da sessão.
    fn rename(&self, id: &str, subject: &str) -> Result<()>;
    /// Define o resumo (`overview`) da sessão.
    ///
    /// Espelho do resumo gerado, para o export/histórico que já leem a coluna.
    fn set_overview(&self, id: &str, overview: &str) -> Result<()>;
    /// Remove a sessão e, em cascata, as transcrições/mensagens dela.
    fn delete(&self, id: &str) -> Result<()>;
}

/// Persistência de segmentos de transcrição (com busca full-text).
///
/// Adaptador: `LocalTranscriptionsRepository`; dobra: `InMemoryTranscriptionsRepository`.
pub trait TranscriptionsRepository: Send + Sync {
    fn insert(&self, segment: &TranscriptSegment) -> Result<()>;
    fn list_by_session(&self, session_id: &str) -> Result<Vec<TranscriptSegment>>;
    fn search(&self, query: &str, limit: usize) -> Result<Vec<TranscriptSegment>>;
    /// Fim da última fala **finalizada** com texto.
    ///
    /// Consulta barata para o agendador saber se há conteúdo novo sem carregar a
    /// sessão inteira a cada tick (BR-028). Usa o **fim** do segmento: o
    /// intervalo do auto-sugerir conta a partir de quando a fala terminou.
    fn latest_final_end_ms(&self, session_id: &str) -> Result<Option<i64>>;

    /// Falas finais recentes, em ordem cronológica, para montar os blocos do
    /// modo classificador (BR-093).
    ///
    /// `since_ms` é comparado com o **fim** do segmento; `limit` fica com as
    /// falas mais recentes (a implementação pode buscar de trás para frente e
    /// devolver em ordem crescente).
    fn list_recent_by_session(
        &self,
        session_id: &str,
        since_ms: i64,
        limit: usize,
    ) -> Result<Vec<TranscriptSegment>>;
}

/// Persistência de mensagens da sessão (sugestões da IA).
///
/// Adaptador: `LocalMessagesRepository`; dobra: `InMemoryMessagesRepository`.
pub trait MessagesRepository: Send + Sync {
    fn insert(&self, message: &ConversationMessage) -> Result<()>;
    fn list_by_session(&self, session_id: &str) -> Result<Vec<ConversationMessage>>;
}
