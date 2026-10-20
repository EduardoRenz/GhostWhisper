//! Porta de gestão das fontes de IA (BR-044, BR-045).
//!
//! Usada pela tela de Configurações: cadastrar, editar, excluir, reordenar,
//! gerenciar a chave de cada fonte e ver como cada uma está.
//!
//! Adaptador: `AiSourceService` (aplicação); dobra de teste:
//! `InMemoryAiSourceService`, `FakeAiSourceService`.

use crate::domain::ai_source::{AiSource, AiSourceHealth};
use crate::error::Result;

/// Gerencia as fontes de IA cadastradas.
///
/// A **lista** (e a sua ordem) é salva pela tela de Configurações junto com
/// as demais settings (`settings_set`) — esta porta não é um segundo caminho
/// de escrita.
pub trait AiSourceService: Send + Sync {
    /// Lista as fontes, **na ordem de prioridade** (a primeira é a primária).
    fn list(&self) -> Result<Vec<AiSource>>;

    /// Informe de saúde de cada fonte conhecida.
    fn health(&self) -> Vec<AiSourceHealth>;

    /// Grava a chave da fonte no armazenamento de segredos.
    ///
    /// Aceita uma fonte que **ainda não foi salva** (rascunho da tela): a
    /// chave vive no keyring, indexada pelo id da fonte.
    fn set_key(&self, source_id: &str, key: &str) -> Result<()>;

    /// `true` quando a fonte tem chave configurada.
    fn has_key(&self, source_id: &str) -> Result<bool>;

    /// Remove a chave da fonte.
    fn clear_key(&self, source_id: &str) -> Result<()>;

    /// Versão mascarada da chave (`••••••••·abcd`); nunca o valor bruto.
    fn key_preview(&self, source_id: &str) -> Result<String>;

    /// Testa a conexão com **uma** fonte ("Validar chave").
    ///
    /// Recebe a fonte inteira (não só o id) para que uma fonte ainda no
    /// rascunho possa ser validada antes de ser salva. A validação bem-sucedida
    /// também **marca a fonte como usada** (a tela mostra "Conectada").
    fn validate(&self, source: &AiSource) -> Result<String>;
}
