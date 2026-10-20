//! Tipos de domínio para chamadas a LLMs.

use serde::{Deserialize, Serialize};

use super::settings::ProviderKind;

/// Configuração resolvida de um provider (com segredo apenas em memória).
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub model: String,
    pub base_url: Option<String>,
    /// Chave de API — nunca persistida pelo núcleo nem registrada em logs.
    pub api_key: String,
}

/// Pedido de completude.
#[derive(Debug, Clone, PartialEq)]
pub struct CompletionRequest {
    pub system: Option<String>,
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl CompletionRequest {
    /// Pedido simples de usuário.
    pub fn user(prompt: impl Into<String>) -> Self {
        Self {
            system: None,
            prompt: prompt.into(),
            max_tokens: None,
            temperature: None,
        }
    }

    /// Define o prompt de sistema.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Limita o número de tokens gerados.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Ajusta a temperatura.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// Uso de tokens de uma completude (o que o provider reporta).
///
/// `reasoning_tokens` só existe em modelos de raciocínio (OpenAI `o*`/`gpt-5`,
/// DeepSeek `deepseek-reasoner`, Gemini `thoughtsTokenCount`); quando o provider
/// não informa, fica `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CompletionUsage {
    /// Tokens do prompt (entrada).
    pub prompt_tokens: u32,
    /// Tokens gerados (saída, **incluindo** o raciocínio).
    pub completion_tokens: u32,
    /// Total (entrada + saída).
    pub total_tokens: u32,
    /// Tokens usados só para raciocínio (quando o provider informa).
    pub reasoning_tokens: Option<u32>,
}

impl CompletionUsage {
    /// `true` quando o modelo consumiu tokens de raciocínio.
    pub fn has_reasoning(&self) -> bool {
        self.reasoning_tokens.unwrap_or(0) > 0
    }
}

/// Resultado de uma completude.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub text: String,
    pub model: String,
    /// Uso de tokens (quando o provider reporta).
    pub usage: Option<CompletionUsage>,
}
