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
    /// Executa uma completude reportando o **texto acumulado** a cada pedaço.
    ///
    /// Implementação padrão: sem streaming — chama [`complete`](Self::complete) e
    /// reporta o texto inteiro uma vez. Providers com SSE sobrescrevem (BR-028).
    fn complete_streaming(
        &self,
        request: &CompletionRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion> {
        let completion = self.complete(request)?;
        on_text(&completion.text);
        Ok(completion)
    }
}
