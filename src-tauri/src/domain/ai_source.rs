//! Fontes de IA cadastradas pelo usuário (BR-044).
//!
//! Uma **fonte** é o par (provedor + modelo + chave). O usuário pode ter
//! várias: a **ordem da lista** é a ordem de prioridade e a primeira é a
//! primária (BR-044). Cada fonte tem um `id` estável (para o keyring e para as
//! rotas por escopo — BR-048) e uma chave **própria**.

use serde::{Deserialize, Serialize};

use crate::domain::settings::ProviderKind;
use crate::error::{GhostWhisperError, Result};

/// Limite de caracteres do rótulo de exibição da fonte.
pub const SOURCE_LABEL_MAX_CHARS: usize = 120;

/// Teto de fontes cadastradas (evita listas gigantes no arquivo de settings).
pub const MAX_SOURCES: usize = 32;

/// Prefixo da chave da fonte no `SecretStore`.
///
/// A chave é **por fonte**, não por tipo de provedor: duas fontes OpenAI com
/// modelos diferentes usam entradas diferentes do keyring (BR-044).
pub const SECRET_KEY_PREFIX: &str = "llm.source.";

/// Chave de uma fonte no `SecretStore` (pelo id).
///
/// Não exige que a fonte já esteja salva: o rascunho da tela de Configurações
/// gera o id, e a chave vive no keyring independentemente do arquivo de
/// settings (BR-044).
pub fn secret_key_for(source_id: &str) -> String {
    format!("{SECRET_KEY_PREFIX}{source_id}")
}

/// Fonte de IA cadastrada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSource {
    /// Identificador estável (uuid v4). Nunca é reaproveitado.
    pub id: String,
    /// Provedor (OpenAI, Gemini, OpenRouter, DeepSeek, custom).
    pub kind: ProviderKind,
    /// Nome de exibição ("OpenAI · Empresa").
    pub label: String,
    /// Modelo usado nas chamadas.
    pub model: String,
    /// Endpoint custom (formato OpenAI). Só é usado por `custom`.
    #[serde(default)]
    pub base_url: Option<String>,
    /// A fonte participa do roteamento?
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl AiSource {
    /// Cria uma fonte com o rótulo padrão do provedor.
    pub fn new(id: impl Into<String>, kind: ProviderKind) -> Self {
        Self {
            id: id.into(),
            kind,
            label: default_label(kind),
            model: kind.default_model().to_string(),
            base_url: None,
            enabled: true,
        }
    }

    /// Chave desta fonte no `SecretStore`.
    pub fn secret_key(&self) -> String {
        secret_key_for(&self.id)
    }

    /// `base_url` efetiva: a explícita ou o padrão do provedor.
    pub fn effective_base_url(&self) -> Option<String> {
        self.base_url
            .clone()
            .filter(|url| !url.trim().is_empty())
            .or_else(|| self.kind.default_base_url().map(str::to_string))
    }

    /// Rótulo de exibição (usa o rótulo do provedor quando vazio).
    pub fn display_label(&self) -> String {
        let trimmed = self.label.trim();
        if trimmed.is_empty() {
            default_label(self.kind)
        } else {
            trimmed.to_string()
        }
    }
}

/// Rótulo padrão de exibição de um provedor.
pub fn default_label(kind: ProviderKind) -> String {
    match kind {
        ProviderKind::OpenAi => "OpenAI".to_string(),
        ProviderKind::Gemini => "Google Gemini".to_string(),
        ProviderKind::OpenRouter => "OpenRouter".to_string(),
        ProviderKind::DeepSeek => "DeepSeek".to_string(),
        ProviderKind::Custom => "Endpoint custom".to_string(),
    }
}

/// Estado de saúde de uma fonte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiSourceState {
    /// Nunca usada (ou estado apagado).
    Unknown,
    /// Respondendo bem.
    Ready,
    /// Falhou, mas ainda é tentada.
    Degraded,
    /// Falhou de vez (circuito aberto).
    Failed,
}

/// Saúde de uma fonte (informe do circuit breaker — BR-045).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSourceHealth {
    /// Fonte a que este informe se refere.
    pub source_id: String,
    /// Estado atual.
    pub state: AiSourceState,
    /// Falhas consecutivas (zera no sucesso).
    pub consecutive_failures: u32,
    /// Latência da última chamada bem-sucedida (ms).
    pub latency_ms: Option<u64>,
    /// Último erro (truncado, **sem** segredo).
    pub last_error: Option<String>,
    /// A fonte está sendo usada como **fallback** (não é a primária da rota).
    pub is_fallback: bool,
    /// Momento (ms) em que o circuito aberto expira.
    pub open_until_ms: Option<i64>,
}

impl AiSourceHealth {
    /// Informe inicial de uma fonte (nunca usada).
    pub fn new(source_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            state: AiSourceState::Unknown,
            consecutive_failures: 0,
            latency_ms: None,
            last_error: None,
            is_fallback: false,
            open_until_ms: None,
        }
    }
}

/// Valida uma fonte isolada.
pub fn validate_source(source: &AiSource) -> Result<()> {
    if source.id.trim().is_empty() {
        return Err(GhostWhisperError::Config(
            "fonte de IA precisa de um id".into(),
        ));
    }
    if source.model.trim().is_empty() {
        return Err(GhostWhisperError::Config(format!(
            "fonte de IA precisa de um modelo: {}",
            source.display_label()
        )));
    }
    if source.label.chars().count() > SOURCE_LABEL_MAX_CHARS {
        return Err(GhostWhisperError::Config(format!(
            "rótulo da fonte excede {SOURCE_LABEL_MAX_CHARS} caracteres"
        )));
    }
    if source.kind.requires_base_url() {
        match source.base_url.as_deref().map(str::trim) {
            Some(url) if url.starts_with("http://") || url.starts_with("https://") => {}
            _ => {
                return Err(GhostWhisperError::Config(
                    "fonte custom exige base_url http(s)".into(),
                ))
            }
        }
    }
    Ok(())
}

/// Valida a lista de fontes (ids únicos e campos válidos).
pub fn validate_sources(sources: &[AiSource]) -> Result<()> {
    if sources.len() > MAX_SOURCES {
        return Err(GhostWhisperError::Config(format!(
            "no máximo {MAX_SOURCES} fontes de IA"
        )));
    }
    let mut seen = std::collections::HashSet::new();
    for source in sources {
        validate_source(source)?;
        if !seen.insert(source.id.as_str()) {
            return Err(GhostWhisperError::Config(format!(
                "id de fonte duplicado: {}",
                source.id
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, kind: ProviderKind) -> AiSource {
        AiSource::new(id, kind)
    }

    #[test]
    fn secret_key_is_per_source_not_per_kind() {
        let a = source("id-a", ProviderKind::OpenAi);
        let b = source("id-b", ProviderKind::OpenAi);
        assert_ne!(a.secret_key(), b.secret_key());
        assert_eq!(a.secret_key(), "llm.source.id-a");
    }

    #[test]
    fn custom_requires_a_base_url() {
        let mut custom = source("id-c", ProviderKind::Custom);
        custom.model = "llama-3.1-8b".into();
        assert!(validate_source(&custom).is_err());

        custom.base_url = Some("http://localhost:11434/v1".into());
        assert!(validate_source(&custom).is_ok());
    }

    #[test]
    fn model_is_required() {
        let mut openai = source("id-a", ProviderKind::OpenAi);
        openai.model = "  ".into();
        assert!(validate_source(&openai).is_err());
    }

    #[test]
    fn rejects_duplicate_ids() {
        let list = vec![
            source("id-a", ProviderKind::OpenAi),
            source("id-a", ProviderKind::Gemini),
        ];
        assert!(validate_sources(&list).is_err());
    }

    #[test]
    fn default_base_url_follows_the_provider() {
        let deepseek = source("id-d", ProviderKind::DeepSeek);
        assert_eq!(
            deepseek.effective_base_url().as_deref(),
            Some("https://api.deepseek.com/v1")
        );
        assert_eq!(deepseek.model, "deepseek-chat");
    }

    #[test]
    fn display_label_falls_back_to_the_provider() {
        let mut custom = source("id-c", ProviderKind::Custom);
        custom.label = "   ".into();
        assert_eq!(custom.display_label(), "Endpoint custom");
        custom.label = "Ollama local".into();
        assert_eq!(custom.display_label(), "Ollama local");
    }

    #[test]
    fn health_starts_unknown() {
        let health = AiSourceHealth::new("id-a");
        assert_eq!(health.state, AiSourceState::Unknown);
        assert!(health.last_error.is_none());
    }
}
