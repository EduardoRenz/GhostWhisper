//! Porta de fábrica de providers de LLM.

use std::sync::Arc;

use crate::domain::llm::ProviderConfig;
use crate::error::Result;
use crate::ports::chat_bot_provider::ChatBotProvider;

/// Constrói o `ChatBotProvider` adequado a uma configuração.
///
/// É o **único** ponto que conhece os adaptadores concretos de provider; a
/// aplicação decide *o quê* usar e injeta esta fábrica.
///
/// Adaptador: `DefaultChatBotProviderFactory`.
pub trait ChatBotProviderFactory: Send + Sync {
    /// Cria o provider para a configuração dada.
    fn create(&self, config: &ProviderConfig) -> Result<Arc<dyn ChatBotProvider>>;
}
