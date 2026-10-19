//! Agentes/modos de sugestão (Reunião, Entrevista Técnica, LeetCode, …).
//!
//! Os **prompts** dos agentes embutidos vivem na camada de aplicação
//! (`application::prompts::assistant`) para o domínio continuar puro. Aqui ficam
//! o tipo, os IDs reservados e as regras de validação — os mesmos para embutidos
//! e personalizados.

use serde::{Deserialize, Serialize};

use crate::error::{GhostWhisperError, Result};

/// Limite de caracteres de cada campo de prompt do agente.
pub const MODE_PROMPT_MAX_CHARS: usize = 16_384;

/// Limite do nome de exibição.
pub const MODE_NAME_MAX_CHARS: usize = 120;

/// IDs dos agentes embutidos (reservados).
pub const BUILTIN_MODE_IDS: [&str; 4] = [
    "meeting",
    "technical_interview",
    "leetcode",
    "system_design",
];

/// Catálogo de **ícones** do agente (identidade visual — BR-037).
///
/// Os desenhos ficam no frontend (`components/AgentIcon.tsx`); aqui fica só a
/// lista de ids válidos, para o domínio validar o que chega das settings.
pub const AGENT_ICON_IDS: [&str; 8] = [
    "sparkle",
    "users",
    "braces",
    "code",
    "layers",
    "book",
    "briefcase",
    "terminal",
];

/// Paleta de **cores** do agente (tokens do design system — BR-037).
///
/// São nomes de token, nunca hex: o tema continua sendo a fonte da verdade.
pub const AGENT_COLOR_TOKENS: [&str; 5] = ["cyan", "violet", "emerald", "coral", "slate"];

/// Ícone padrão de um agente sem identidade definida.
pub const DEFAULT_AGENT_ICON: &str = "sparkle";

/// Cor padrão de um agente sem identidade definida.
pub const DEFAULT_AGENT_COLOR: &str = "slate";

/// Ícone padrão (settings antigas sem o campo).
pub fn default_agent_icon() -> String {
    DEFAULT_AGENT_ICON.to_string()
}

/// Cor padrão (settings antigas sem o campo).
pub fn default_agent_color() -> String {
    DEFAULT_AGENT_COLOR.to_string()
}

/// `true` quando o id de ícone existe no catálogo.
pub fn is_valid_agent_icon(icon: &str) -> bool {
    AGENT_ICON_IDS.contains(&icon)
}

/// `true` quando o token de cor existe na paleta.
pub fn is_valid_agent_color(color: &str) -> bool {
    AGENT_COLOR_TOKENS.contains(&color)
}

/// Agente de sugestão: embutido ou personalizado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantMode {
    /// Identificador estável (`meeting`, `custom-…`).
    pub id: String,
    /// Nome exibido na UI.
    pub name: String,
    /// Categoria livre (`productivity`, `engineering`, `custom`, …).
    #[serde(default)]
    pub category: String,
    /// Contrato de respostas escritas e critérios de domínio.
    #[serde(default)]
    pub system_prompt: String,
    /// Contrato da sugestão ao vivo (o que a janela mostra).
    #[serde(default)]
    pub suggestion_prompt: String,
    /// Contexto fixo do agente (texto suplementar).
    #[serde(default)]
    pub context: String,
    /// Ícone do agente (catálogo — BR-037).
    #[serde(default = "default_agent_icon")]
    pub icon: String,
    /// Cor do agente (token do design system — BR-037).
    #[serde(default = "default_agent_color")]
    pub color: String,
}

impl AssistantMode {
    /// `true` quando o id pertence a um agente embutido.
    #[cfg(test)]
    pub fn is_builtin(&self) -> bool {
        is_builtin_id(&self.id)
    }

    /// Normaliza a identidade (ícone/cor) para valores do catálogo.
    ///
    /// Usado ao carregar settings antigas ou edited à mão: um ícone
    /// desconhecido vira o padrão em vez de derrubar o agente inteiro.
    pub fn with_valid_identity(mut self) -> Self {
        if !is_valid_agent_icon(&self.icon) {
            self.icon = default_agent_icon();
        }
        if !is_valid_agent_color(&self.color) {
            self.color = default_agent_color();
        }
        self
    }
}

/// Indica se `id` é um dos agentes embutidos.
pub fn is_builtin_id(id: &str) -> bool {
    BUILTIN_MODE_IDS.contains(&id)
}

/// Valida um agente personalizado.
pub fn validate_mode(mode: &AssistantMode) -> Result<()> {
    if mode.id.trim().is_empty() {
        return Err(GhostWhisperError::Config(
            "agente precisa de um id".to_string(),
        ));
    }
    if is_builtin_id(&mode.id) {
        return Err(GhostWhisperError::Config(format!(
            "id de agente embutido é reservado: {}",
            mode.id
        )));
    }
    let name = mode.name.trim();
    if name.is_empty() {
        return Err(GhostWhisperError::Config(
            "agente precisa de um nome".to_string(),
        ));
    }
    if name.chars().count() > MODE_NAME_MAX_CHARS {
        return Err(GhostWhisperError::Config(format!(
            "nome do agente excede {MODE_NAME_MAX_CHARS} caracteres"
        )));
    }
    if mode.category.chars().count() > 60 {
        return Err(GhostWhisperError::Config(
            "categoria do agente excede 60 caracteres".to_string(),
        ));
    }
    for (field, value) in [
        ("prompt de sistema", &mode.system_prompt),
        ("prompt de sugestão", &mode.suggestion_prompt),
        ("contexto", &mode.context),
    ] {
        if value.chars().count() > MODE_PROMPT_MAX_CHARS {
            return Err(GhostWhisperError::Config(format!(
                "{field} do agente excede {MODE_PROMPT_MAX_CHARS} caracteres"
            )));
        }
    }
    if mode.system_prompt.trim().is_empty()
        && mode.suggestion_prompt.trim().is_empty()
        && mode.context.trim().is_empty()
    {
        return Err(GhostWhisperError::Config(
            "agente precisa de ao menos um prompt ou contexto".to_string(),
        ));
    }
    if !is_valid_agent_icon(&mode.icon) {
        return Err(GhostWhisperError::Config(format!(
            "ícone de agente desconhecido: {}",
            mode.icon
        )));
    }
    if !is_valid_agent_color(&mode.color) {
        return Err(GhostWhisperError::Config(format!(
            "cor de agente desconhecida: {}",
            mode.color
        )));
    }
    Ok(())
}

/// Valida a lista de agentes personalizados (ids únicos e campos válidos).
pub fn validate_custom_modes(modes: &[AssistantMode]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for mode in modes {
        validate_mode(mode)?;
        if !seen.insert(mode.id.as_str()) {
            return Err(GhostWhisperError::Config(format!(
                "id de agente duplicado: {}",
                mode.id
            )));
        }
    }
    Ok(())
}

/// Procura um agente por id, priorizando os personalizados.
pub fn find_mode<'a>(
    id: &str,
    custom: &'a [AssistantMode],
    builtins: &'a [AssistantMode],
) -> Option<&'a AssistantMode> {
    custom
        .iter()
        .chain(builtins.iter())
        .find(|mode| mode.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(id: &str, name: &str) -> AssistantMode {
        AssistantMode {
            id: id.to_string(),
            name: name.to_string(),
            category: "custom".to_string(),
            system_prompt: "Você é um agente de vendas.".to_string(),
            suggestion_prompt: "Sugira a próxima fala.".to_string(),
            context: String::new(),
            icon: default_agent_icon(),
            color: default_agent_color(),
        }
    }

    #[test]
    fn builtin_ids_are_reserved() {
        for id in BUILTIN_MODE_IDS {
            assert!(is_builtin_id(id));
            let mode = custom(id, "x");
            assert!(validate_mode(&mode).is_err());
        }
        assert!(!is_builtin_id("custom-vendas"));
    }

    #[test]
    fn valid_custom_mode_passes() {
        assert!(validate_mode(&custom("custom-vendas", "Vendas")).is_ok());
    }

    #[test]
    fn rejects_empty_name_and_empty_prompts() {
        let mut mode = custom("custom-x", "  ");
        assert!(validate_mode(&mode).is_err());

        mode = custom("custom-x", "X");
        mode.system_prompt.clear();
        mode.suggestion_prompt.clear();
        mode.context.clear();
        assert!(validate_mode(&mode).is_err());
    }

    #[test]
    fn rejects_duplicate_ids() {
        let modes = vec![custom("custom-a", "A"), custom("custom-a", "Outro A")];
        assert!(validate_custom_modes(&modes).is_err());
    }

    #[test]
    fn finds_custom_before_builtin() {
        let custom_modes = vec![custom("custom-a", "A")];
        let builtins = vec![custom("meeting", "Reunião")];
        assert_eq!(
            find_mode("meeting", &custom_modes, &builtins).map(|mode| mode.name.as_str()),
            Some("Reunião")
        );
        assert_eq!(
            find_mode("custom-a", &custom_modes, &builtins).map(|mode| mode.name.as_str()),
            Some("A")
        );
        assert!(find_mode("missing", &custom_modes, &builtins).is_none());
    }

    #[test]
    fn serializes_the_shape_the_frontend_expects() {
        let json = serde_json::to_value(custom("custom-a", "A")).expect("json");
        assert_eq!(json["id"], "custom-a");
        assert_eq!(json["name"], "A");
        assert_eq!(json["category"], "custom");
        assert!(json["system_prompt"].is_string());
        assert!(json["suggestion_prompt"].is_string());
        assert!(json["context"].is_string());
        assert_eq!(json["icon"], DEFAULT_AGENT_ICON);
        assert_eq!(json["color"], DEFAULT_AGENT_COLOR);
    }

    #[test]
    fn old_settings_without_identity_load_with_the_defaults() {
        // Settings gravadas antes da identidade (BR-037) não têm `icon`/`color`.
        let json = serde_json::json!({
            "id": "custom-a",
            "name": "A",
            "category": "custom",
            "system_prompt": "prompt",
            "suggestion_prompt": "sugestão",
            "context": "",
        });
        let mode: AssistantMode = serde_json::from_value(json).expect("mode");
        assert_eq!(mode.icon, DEFAULT_AGENT_ICON);
        assert_eq!(mode.color, DEFAULT_AGENT_COLOR);
        assert!(validate_mode(&mode).is_ok());
    }

    #[test]
    fn rejects_unknown_icon_and_color() {
        let mut mode = custom("custom-a", "A");
        mode.icon = "rocket".to_string();
        assert!(validate_mode(&mode).is_err());

        let mut mode = custom("custom-a", "A");
        mode.color = "#00f2fe".to_string();
        assert!(validate_mode(&mode).is_err());
    }

    #[test]
    fn normalizes_an_unknown_identity_to_the_defaults() {
        let mut mode = custom("custom-a", "A");
        mode.icon = "rocket".to_string();
        mode.color = "chartreuse".to_string();
        let mode = mode.with_valid_identity();
        assert_eq!(mode.icon, DEFAULT_AGENT_ICON);
        assert_eq!(mode.color, DEFAULT_AGENT_COLOR);
    }

    #[test]
    fn every_catalog_icon_and_color_is_accepted() {
        for icon in AGENT_ICON_IDS {
            let mut mode = custom("custom-a", "A");
            mode.icon = icon.to_string();
            assert!(validate_mode(&mode).is_ok(), "ícone recusado: {icon}");
        }
        for color in AGENT_COLOR_TOKENS {
            let mut mode = custom("custom-a", "A");
            mode.color = color.to_string();
            assert!(validate_mode(&mode).is_ok(), "cor recusada: {color}");
        }
    }
}
