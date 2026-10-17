//! Ações acionáveis por atalhos globais e seus vínculos.

use serde::{Deserialize, Serialize};

/// Ação disparada por um atalho global.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    ToggleOverlay,
    ToggleSession,
    ToggleStealth,
    RequestInsight,
}

impl ShortcutAction {
    /// Valor persistido/exposto.
    pub fn as_str(&self) -> &'static str {
        match self {
            ShortcutAction::ToggleOverlay => "toggle_overlay",
            ShortcutAction::ToggleSession => "toggle_session",
            ShortcutAction::ToggleStealth => "toggle_stealth",
            ShortcutAction::RequestInsight => "request_insight",
        }
    }

    /// Todas as ações, na ordem de exibição.
    pub fn all() -> [ShortcutAction; 4] {
        [
            ShortcutAction::ToggleOverlay,
            ShortcutAction::ToggleSession,
            ShortcutAction::ToggleStealth,
            ShortcutAction::RequestInsight,
        ]
    }

    /// Rótulo legível (UI).
    pub fn label(&self) -> &'static str {
        match self {
            ShortcutAction::ToggleOverlay => "Mostrar/ocultar overlay",
            ShortcutAction::ToggleSession => "Iniciar/encerrar sessão",
            ShortcutAction::ToggleStealth => "Alternar stealth",
            ShortcutAction::RequestInsight => "Pedir sugestão",
        }
    }

    /// Acelerador padrão.
    pub fn default_accelerator(&self) -> &'static str {
        match self {
            ShortcutAction::ToggleOverlay => "CommandOrControl+Shift+O",
            ShortcutAction::ToggleSession => "CommandOrControl+Shift+S",
            ShortcutAction::ToggleStealth => "CommandOrControl+Shift+H",
            ShortcutAction::RequestInsight => "CommandOrControl+Shift+I",
        }
    }
}

/// Vínculo entre uma ação e um acelerador (ex.: `CommandOrControl+Shift+O`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutBinding {
    pub action: ShortcutAction,
    pub accelerator: String,
}

/// Atalho que não pôde ser registrado (já em uso/inválido).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutConflict {
    pub action: ShortcutAction,
    pub accelerator: String,
    pub reason: String,
}

/// Vínculos padrão (um por ação).
pub fn default_bindings() -> Vec<ShortcutBinding> {
    ShortcutAction::all()
        .iter()
        .map(|action| ShortcutBinding {
            action: *action,
            accelerator: action.default_accelerator().to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cover_all_actions() {
        let bindings = default_bindings();
        assert_eq!(bindings.len(), ShortcutAction::all().len());
        for action in ShortcutAction::all() {
            assert!(bindings.iter().any(|binding| binding.action == action));
        }
    }

    #[test]
    fn actions_have_unique_ids_and_labels() {
        let mut ids: Vec<&str> = ShortcutAction::all().iter().map(|a| a.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ShortcutAction::all().len());
        for action in ShortcutAction::all() {
            assert!(!action.label().is_empty());
            assert!(action.default_accelerator().contains("CommandOrControl"));
        }
    }

    #[test]
    fn binding_serializes_action_as_snake_case() {
        let json = serde_json::to_string(&ShortcutBinding {
            action: ShortcutAction::ToggleOverlay,
            accelerator: "CommandOrControl+Shift+O".into(),
        })
        .expect("serializa");
        assert!(json.contains("toggle_overlay"));
    }
}
