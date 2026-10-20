//! Dobra de teste: provider determinístico (sem rede).

use std::sync::Mutex;

use crate::domain::llm::{Completion, CompletionRequest};
use crate::error::Result;
use crate::ports::chat_bot_provider::ChatBotProvider;

/// `ChatBotProvider` de teste: responde um texto fixo.
pub struct FakeChatBotProvider {
    response: Mutex<String>,
}

impl FakeChatBotProvider {
    /// Cria o fake com a resposta padrão `"ok"`.
    pub fn new() -> Self {
        Self {
            response: Mutex::new("ok".to_string()),
        }
    }

    /// Define a resposta devolvida.
    pub fn with_response(self, response: impl Into<String>) -> Self {
        if let Ok(mut guard) = self.response.lock() {
            *guard = response.into();
        }
        self
    }
}

impl Default for FakeChatBotProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatBotProvider for FakeChatBotProvider {
    fn id(&self) -> String {
        "fake:test".to_string()
    }

    fn complete(&self, _request: &CompletionRequest) -> Result<Completion> {
        let text = self
            .response
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        Ok(Completion {
            text,
            model: "fake".to_string(),
            usage: None,
        })
    }
}
