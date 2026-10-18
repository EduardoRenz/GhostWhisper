//! Porta do caso de uso de sugestão (inbound port).
//!
//! Substitui o antigo `InsightGenerator`: agora a sugestão tem **modo/agente**,
//! pode ser efêmera (janela) ou persistida (chat/atalho/fixar) e sabe se pulou
//! um tick sem fala nova.

use serde::Serialize;

use crate::error::Result;

/// Pedido de sugestão.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SuggestionRequest {
    /// Sessão alvo (`None` = ativa ou a mais recente).
    pub session_id: Option<String>,
    /// Agente (`None` = o selecionado nas configurações).
    pub mode_id: Option<String>,
    /// `true` grava a sugestão em `messages` (chat/atalho/fixar).
    pub persist: bool,
    /// Último instante já considerado (`None` força uma nova análise).
    pub since_ms: Option<i64>,
    /// Sugestão anterior, para o modelo não repetir.
    pub last_suggestion: Option<String>,
    /// Contexto colado pelo usuário (fonte suplementar, nunca instrução).
    pub extra_context: Option<String>,
}

impl SuggestionRequest {
    /// Pedido persistido para o chat/atalho, com o agente selecionado.
    pub fn persist(session_id: Option<String>) -> Self {
        Self {
            session_id,
            persist: true,
            ..Self::default()
        }
    }

    /// Pedido efêmero para a janela de sugestões.
    pub fn ephemeral(session_id: Option<String>, mode_id: String) -> Self {
        Self {
            session_id,
            mode_id: Some(mode_id),
            persist: false,
            ..Self::default()
        }
    }

    /// Define o instante mínimo considerado.
    pub fn with_since_ms(mut self, since_ms: Option<i64>) -> Self {
        self.since_ms = since_ms;
        self
    }

    /// Inclui a sugestão anterior.
    pub fn with_last_suggestion(mut self, last_suggestion: Option<String>) -> Self {
        self.last_suggestion = last_suggestion.filter(|text| !text.trim().is_empty());
        self
    }

    /// Inclui o contexto extra do usuário.
    pub fn with_extra_context(mut self, extra_context: Option<String>) -> Self {
        self.extra_context = extra_context.filter(|text| !text.trim().is_empty());
        self
    }
}

/// Sugestão gerada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Suggestion {
    pub session_id: String,
    /// Id do agente que gerou.
    pub mode_id: String,
    /// Nome do agente (rótulo da UI).
    pub mode_name: String,
    /// Markdown sugerido.
    pub text: String,
    /// Milissegundos desde o início da sessão.
    pub at_ms: i64,
    /// Início da última fala considerada (cursor do próximo pedido).
    pub context_until_ms: i64,
    /// Id da mensagem gravada (quando `persist`/`pin`).
    pub message_id: Option<String>,
    /// `true` quando não havia fala nova e o LLM não foi chamado.
    pub skipped: bool,
}

/// Pedido de fixação de uma sugestão já gerada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinSuggestionRequest {
    pub session_id: String,
    pub mode_id: String,
    pub text: String,
    /// Instante original da sugestão (ms desde o início da sessão).
    pub at_ms: i64,
}

/// Gera sugestões contextuais por agente.
///
/// Implementado pela aplicação (`SuggestionService`) e chamado pelo IPC.
pub trait SuggestionGenerator: Send + Sync {
    /// Gera (e opcionalmente persiste) uma sugestão.
    fn generate(&self, request: SuggestionRequest) -> Result<Suggestion>;
    /// Persiste uma sugestão já gerada, sem chamar o LLM.
    fn pin(&self, request: PinSuggestionRequest) -> Result<Suggestion>;
}
