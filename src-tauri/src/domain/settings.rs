//! Configurações do usuário (não sensíveis) e catálogo de providers.

use serde::{Deserialize, Serialize};

use crate::domain::shortcuts::ShortcutBinding;
use crate::error::{GhostWhisperError, Result};

/// Idiomas suportados em origem e destino.
///
/// `auto` na origem = detecção; `auto` no destino = **não traduzir**.
pub const SUPPORTED_LANGUAGES: [&str; 3] = ["auto", "pt", "en"];

/// Indica se o código de idioma é suportado.
pub fn is_supported_language(code: &str) -> bool {
    SUPPORTED_LANGUAGES.contains(&code.trim())
}

/// Nome legível do idioma, usado nos prompts do LLM.
pub fn language_name(code: &str) -> &'static str {
    match code.trim() {
        "pt" => "português",
        "en" => "inglês",
        _ => "o mesmo idioma da transcrição",
    }
}

/// Provider de LLM suportado (BYO key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    OpenAi,
    Gemini,
    OpenRouter,
    Custom,
}

impl ProviderKind {
    /// Valor persistido/exposto.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "openai",
            ProviderKind::Gemini => "gemini",
            ProviderKind::OpenRouter => "openrouter",
            ProviderKind::Custom => "custom",
        }
    }

    /// Todos os providers, na ordem de exibição.
    pub fn all() -> [ProviderKind; 4] {
        [
            ProviderKind::OpenAi,
            ProviderKind::Gemini,
            ProviderKind::OpenRouter,
            ProviderKind::Custom,
        ]
    }

    /// Modelo sugerido por provider.
    pub fn default_model(&self) -> &'static str {
        match self {
            ProviderKind::OpenAi => "gpt-4o-mini",
            ProviderKind::Gemini => "gemini-2.0-flash",
            ProviderKind::OpenRouter => "openai/gpt-4o-mini",
            ProviderKind::Custom => "",
        }
    }

    /// `base_url` padrão (OpenAI-compatível) quando aplicável.
    pub fn default_base_url(&self) -> Option<&'static str> {
        match self {
            ProviderKind::OpenAi => Some("https://api.openai.com/v1"),
            ProviderKind::OpenRouter => Some("https://openrouter.ai/api/v1"),
            ProviderKind::Gemini | ProviderKind::Custom => None,
        }
    }

    /// `base_url` é obrigatória para endpoints custom.
    pub fn requires_base_url(&self) -> bool {
        matches!(self, ProviderKind::Custom)
    }

    /// Chave no `SecretStore` associada ao provider.
    pub fn secret_key(&self) -> String {
        format!("llm.{}", self.as_str())
    }
}

/// Configurações persistidas em disco (nunca contêm segredos).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub provider: ProviderKind,
    pub model: String,
    /// Endpoint custom (formato OpenAI). Obrigatório quando `provider = custom`.
    pub base_url: Option<String>,
    /// Idioma de origem da transcrição (`auto` para detecção).
    pub source_language: String,
    /// Idioma de destino da tradução.
    pub target_language: String,
    pub translation_enabled: bool,
    /// Modelo whisper.cpp local (`tiny`, `base`, ...).
    pub whisper_model: String,
    /// Ganho aplicado ao microfone antes da transcrição (1×–16×).
    ///
    /// O áudio do sistema nunca é amplificado. Ver BR-018 e
    /// [`crate::domain::audio::apply_gain`].
    #[serde(default = "default_mic_gain")]
    pub mic_gain: f32,
    /// Modo stealth (sem barra de tarefas / sem título).
    #[serde(default)]
    pub stealth: bool,
    /// Vínculos de atalhos globais.
    #[serde(default = "crate::domain::shortcuts::default_bindings")]
    pub shortcuts: Vec<ShortcutBinding>,
    /// Posição do overlay na tela (lembrada após o usuário mover).
    #[serde(default)]
    pub overlay_position: Option<(i32, i32)>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: ProviderKind::OpenAi,
            model: ProviderKind::OpenAi.default_model().to_string(),
            base_url: None,
            source_language: "auto".to_string(),
            target_language: "pt".to_string(),
            translation_enabled: false,
            whisper_model: "tiny".to_string(),
            mic_gain: default_mic_gain(),
            stealth: false,
            shortcuts: crate::domain::shortcuts::default_bindings(),
            overlay_position: None,
        }
    }
}

/// Ganho de microfone padrão (Perssua: `local_whisper_user_mic_gain = 8`).
fn default_mic_gain() -> f32 {
    crate::domain::audio::DEFAULT_MIC_GAIN
}

impl Settings {
    /// Valida invariantes antes de persistir/usar.
    pub fn validate(&self) -> Result<()> {
        if self.model.trim().is_empty() {
            return Err(GhostWhisperError::Config(
                "modelo não pode ser vazio".into(),
            ));
        }
        if self.provider.requires_base_url() {
            match self.base_url.as_deref().map(str::trim) {
                Some(url) if url.starts_with("http://") || url.starts_with("https://") => {}
                _ => {
                    return Err(GhostWhisperError::Config(
                        "provider custom exige base_url http(s)".into(),
                    ))
                }
            }
        }
        if self.target_language.trim().is_empty() {
            return Err(GhostWhisperError::Config(
                "idioma de destino não pode ser vazio".into(),
            ));
        }
        if !is_supported_language(&self.source_language) {
            return Err(GhostWhisperError::Config(format!(
                "idioma de origem não suportado: {} (use auto, pt ou en)",
                self.source_language
            )));
        }
        if !is_supported_language(&self.target_language) {
            return Err(GhostWhisperError::Config(format!(
                "idioma de destino não suportado: {} (use auto, pt ou en)",
                self.target_language
            )));
        }
        Ok(())
    }

    /// Corrige valores fora do contrato (ex.: settings antigas com idioma livre).
    pub fn sanitized(mut self) -> Self {
        if !is_supported_language(&self.source_language) {
            self.source_language = "auto".to_string();
        }
        if !is_supported_language(&self.target_language) {
            self.target_language = "pt".to_string();
        }
        self.mic_gain = crate::domain::audio::clamp_mic_gain(self.mic_gain);
        self
    }

    /// `base_url` efetiva para providers OpenAI-compatíveis.
    pub fn effective_base_url(&self) -> Option<String> {
        self.base_url
            .clone()
            .filter(|url| !url.trim().is_empty())
            .or_else(|| self.provider.default_base_url().map(str::to_string))
    }
}

/// Mascara um segredo para exibição na UI (nunca devolve o valor bruto).
///
/// Regra de produto: o campo mostra `••••••••·abcd` (8 balas + últimos 4
/// caracteres) para indicar que a chave **está lá**; chaves curtas (< 8) não
/// expõem o sufixo, e chaves vazias viram string vazia.
pub fn redact_secret(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.chars().count() < 8 {
        if trimmed.is_empty() {
            return String::new();
        }
        return "••••••••".to_string();
    }
    let tail: String = trimmed.chars().skip(trimmed.chars().count() - 4).collect();
    format!("••••••••·{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_round_trips() {
        for kind in ProviderKind::all() {
            let json = serde_json::to_string(&kind).expect("serializa");
            let parsed: ProviderKind = serde_json::from_str(&json).expect("desserializa");
            assert_eq!(parsed, kind);
            assert!(!kind.as_str().is_empty());
        }
        assert!(serde_json::from_str::<ProviderKind>("\"nope\"").is_err());
    }

    #[test]
    fn default_settings_are_valid() {
        let settings = Settings::default();
        assert!(settings.validate().is_ok());
        assert!(!settings.translation_enabled);
    }

    #[test]
    fn custom_requires_base_url() {
        let missing_base_url = Settings {
            provider: ProviderKind::Custom,
            model: "llama-3.1-8b".into(),
            base_url: None,
            ..Settings::default()
        };
        assert!(missing_base_url.validate().is_err());

        let with_base_url = Settings {
            base_url: Some("https://ollama.local/v1".into()),
            ..missing_base_url
        };
        assert!(with_base_url.validate().is_ok());
    }

    #[test]
    fn openai_has_default_base_url() {
        let settings = Settings::default();
        assert_eq!(
            settings.effective_base_url().as_deref(),
            Some("https://api.openai.com/v1")
        );
    }

    #[test]
    fn settings_serialize_without_secret_fields() {
        let json = serde_json::to_string(&Settings::default()).expect("serializa");
        assert!(!json.contains("key"));
        assert!(!json.contains("secret"));
    }

    #[test]
    fn redact_secret_hides_value_but_shows_tail() {
        let masked = redact_secret("sk-proj-abcdefgh-1234");
        assert_eq!(masked, "••••••••·1234");
        // Nunca devolve o segredo inteiro nem a maior parte dele.
        assert!(!masked.contains("sk-proj"));
        assert!(!masked.contains("abcdefgh"));
    }

    #[test]
    fn redact_secret_hides_short_keys_completely() {
        assert_eq!(redact_secret("abc"), "••••••••");
        assert_eq!(redact_secret("  "), "");
        assert_eq!(redact_secret(""), "");
    }

    #[test]
    fn rejects_unsupported_languages() {
        let source = Settings {
            source_language: "es".into(),
            ..Settings::default()
        };
        assert!(source.validate().is_err());

        let target = Settings {
            target_language: "de".into(),
            ..Settings::default()
        };
        assert!(target.validate().is_err());

        let ok = Settings {
            source_language: "pt".into(),
            target_language: "en".into(),
            ..Settings::default()
        };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn language_names_are_readable() {
        assert_eq!(language_name("pt"), "português");
        assert_eq!(language_name("en"), "inglês");
        assert_eq!(language_name("auto"), "o mesmo idioma da transcrição");
        assert!(is_supported_language("auto"));
        assert!(!is_supported_language("fr"));
    }

    #[test]
    fn sanitized_coerces_unsupported_languages() {
        let dirty = Settings {
            source_language: "es".into(),
            target_language: "de".into(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(dirty.source_language, "auto");
        assert_eq!(dirty.target_language, "pt");
        assert!(dirty.validate().is_ok());
    }

    #[test]
    fn mic_gain_defaults_to_the_perssua_value() {
        // BR-018: ganho padrão do Perssua (local_whisper_user_mic_gain = 8).
        assert_eq!(Settings::default().mic_gain, 8.0);
    }

    #[test]
    fn sanitized_clamps_mic_gain() {
        let too_loud = Settings {
            mic_gain: 99.0,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(too_loud.mic_gain, crate::domain::audio::MAX_MIC_GAIN);

        let too_quiet = Settings {
            mic_gain: 0.1,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(too_quiet.mic_gain, crate::domain::audio::MIN_MIC_GAIN);

        let broken = Settings {
            mic_gain: f32::NAN,
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(broken.mic_gain, crate::domain::audio::DEFAULT_MIC_GAIN);
    }

    #[test]
    fn mic_gain_survives_a_round_trip_and_old_settings_get_the_default() {
        let json = serde_json::to_string(&Settings::default()).expect("serializa");
        let parsed: Settings = serde_json::from_str(&json).expect("desserializa");
        assert_eq!(parsed.mic_gain, 8.0);

        // Config antiga (sem o campo) cai no padrão em vez de falhar.
        let legacy = json.replace("\"mic_gain\":8.0,", "");
        assert!(!legacy.contains("mic_gain"));
        let parsed: Settings = serde_json::from_str(&legacy).expect("desserializa antigo");
        assert_eq!(parsed.mic_gain, 8.0);
    }
}
