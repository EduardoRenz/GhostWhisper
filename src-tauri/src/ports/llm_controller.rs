//! Porta de controle de LLM (inbound port / caso de uso).

use crate::domain::llm::{Completion, CompletionRequest};
use crate::error::Result;

/// Valida a configuração atual e executa completudes.
///
/// Implementado pela aplicação (`LlmRouter`) e chamado pelo IPC.
pub trait LlmController: Send + Sync {
    /// Rótulo do provider/modelo ativo (sem segredos).
    fn provider_label(&self) -> Result<String>;
    /// Testa a conexão ("validar chave"); retorna a resposta do provider.
    fn validate(&self) -> Result<String>;
    /// Executa uma completude com a configuração ativa.
    fn complete(&self, request: &CompletionRequest) -> Result<Completion>;

    /// Executa uma completude reportando o **texto acumulado** a cada pedaço.
    ///
    /// Implementação padrão: sem streaming — chama [`complete`](Self::complete) e
    /// reporta o texto inteiro uma única vez. Providers com SSE sobrescrevem para
    /// a UI mostrar a sugestão crescendo (BR-028).
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
