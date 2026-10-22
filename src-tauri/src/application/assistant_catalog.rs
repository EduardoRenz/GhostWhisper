//! Catálogo de agentes: defaults materializados (BR-074/BR-075).
//!
//! O `Settings::default()` do **domínio** nasce puro (sem agentes); o catálogo
//! de fábrica vive na aplicação. Este módulo compõe o default que o
//! "Restaurar Padrões" entrega à UI — o default do domínio **mais** os agentes
//! de fábrica, com a marca de semeadura ligada.

use crate::application::prompts::assistant::factory_modes;
use crate::domain::settings::Settings;

/// Defaults do app com os agentes de fábrica materializados.
pub fn default_settings() -> Settings {
    Settings {
        custom_assistant_modes: factory_modes(),
        assistant_modes_seeded: true,
        ..Settings::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_carry_the_factory_agents() {
        let settings = default_settings();
        assert!(settings.assistant_modes_seeded);
        let ids: Vec<&str> = settings
            .custom_assistant_modes
            .iter()
            .map(|mode| mode.id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec![
                "meeting",
                "technical_interview",
                "leetcode",
                "system_design"
            ]
        );
        for mode in &settings.custom_assistant_modes {
            assert!(!mode.summary_prompt.is_empty());
        }
        // O default materializado é aceito pelo save.
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn the_pure_default_has_no_agents() {
        // Garante que o domínio continua puro (R1): quem semeia é a aplicação.
        let settings = Settings::default();
        assert!(settings.custom_assistant_modes.is_empty());
        assert!(!settings.assistant_modes_seeded);
    }
}
