//! Tipos de domínio para chamadas a LLMs.

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

/// Resultado de uma completude.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub text: String,
    pub model: String,
}
