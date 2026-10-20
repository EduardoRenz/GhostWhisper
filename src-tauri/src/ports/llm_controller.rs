//! Porta de controle de LLM (inbound port / caso de uso).

use crate::domain::ai_route::AiRouteScope;
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

    /// Executa uma completude priorizando a fonte vinculada ao **escopo**
    /// (BR-048).
    ///
    /// Sem rota configurada, cai na fonte primária — o comportamento de
    /// sempre. A implementação padrão delega a [`complete`](Self::complete).
    fn complete_for(
        &self,
        _scope: &AiRouteScope,
        request: &CompletionRequest,
    ) -> Result<Completion> {
        self.complete(request)
    }

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

    /// Executa uma completude com streaming priorizando o **escopo** (BR-048).
    ///
    /// Implementação padrão: delega a [`complete_streaming`](Self::complete_streaming).
    fn complete_streaming_for(
        &self,
        scope: &AiRouteScope,
        request: &CompletionRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<Completion> {
        let _ = scope;
        self.complete_streaming(request, on_text)
    }
}
