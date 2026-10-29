//! Tipos de domínio para chamadas a LLMs.

use serde::{Deserialize, Serialize};
use std::time::Duration;

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

/// Uma imagem anexada a um pedido (print de tela, BR-056).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionImage {
    /// Tipo MIME (ex.: `image/jpeg`).
    pub mime: String,
    /// Bytes da imagem (já comprimidos).
    pub data: Vec<u8>,
}

/// Pedido de completude.
#[derive(Debug, Clone, PartialEq)]
pub struct CompletionRequest {
    pub system: Option<String>,
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    /// Imagens anexadas (vazio = pedido só de texto — o caso comum).
    pub images: Vec<CompletionImage>,
    /// Prazo que o chamador aceita esperar (BR-045).
    ///
    /// O adaptador aplica o limite na chamada HTTP; `None` deixa o padrão do
    /// adaptador. Quem decide o valor é a camada de aplicação
    /// (`application::llm_timeouts`), não o domínio.
    pub timeout: Option<Duration>,
}

impl CompletionRequest {
    /// Pedido simples de usuário.
    pub fn user(prompt: impl Into<String>) -> Self {
        Self {
            system: None,
            prompt: prompt.into(),
            max_tokens: None,
            temperature: None,
            images: Vec::new(),
            timeout: None,
        }
    }

    /// Define o prazo de espera (sobrescreve um valor anterior).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Define o prazo **só se** ninguém definiu um: o chamador manda.
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        if self.timeout.is_none() {
            self.timeout = Some(timeout);
        }
        self
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

    /// Anexa imagens ao pedido (BR-056).
    pub fn with_images(mut self, images: Vec<CompletionImage>) -> Self {
        self.images = images;
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
    /// Provedor que respondeu (`openai`, `gemini`, `deepseek`, …).
    ///
    /// `None` quando o provider não soube dizer.
    pub provider: Option<String>,
    /// Uso de tokens (quando o provider reporta).
    pub usage: Option<CompletionUsage>,
}
