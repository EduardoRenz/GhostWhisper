//! Porta de provider de LLM (`ChatBotProvider`).

use crate::domain::llm::{Completion, CompletionRequest};
use crate::error::Result;

/// Gera respostas/insights a partir de um prompt.
///
/// Adaptadores: `OpenAiChatBotProvider`, `GeminiChatBotProvider`,
/// `OpenRouterChatBotProvider`, `CustomChatBotProvider`, `FakeChatBotProvider`.
pub trait ChatBotProvider: Send + Sync {
    /// Identificador legível do provider/modelo (sem segredos).
    fn id(&self) -> String;
    /// Executa uma completude.
    fn complete(&self, request: &CompletionRequest) -> Result<Completion>;
}
