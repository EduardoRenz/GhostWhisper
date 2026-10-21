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
    /// Leva o foco para a barra "Perguntar" (BR-052).
    FocusAsk,
    /// Captura a tela e pergunta (BR-055).
    CaptureScreen,
    /// Inicia/pausa/retoma a gravação da sessão (BR-065).
    RecordingToggle,
}

impl ShortcutAction {
    /// Valor persistido/exposto.
    pub fn as_str(&self) -> &'static str {
        match self {
            ShortcutAction::ToggleOverlay => "toggle_overlay",
            ShortcutAction::ToggleSession => "toggle_session",
            ShortcutAction::ToggleStealth => "toggle_stealth",
            ShortcutAction::RequestInsight => "request_insight",
            ShortcutAction::FocusAsk => "focus_ask",
            ShortcutAction::CaptureScreen => "capture_screen",
            ShortcutAction::RecordingToggle => "recording_toggle",
        }
    }

    /// Todas as ações, na ordem de exibição.
    pub fn all() -> [ShortcutAction; 7] {
        [
            ShortcutAction::FocusAsk,
            ShortcutAction::CaptureScreen,
            ShortcutAction::ToggleOverlay,
            ShortcutAction::ToggleSession,
            ShortcutAction::RecordingToggle,
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
            ShortcutAction::FocusAsk => "Perguntar ao GhostWhisper",
            ShortcutAction::CaptureScreen => "Capturar a tela e perguntar",
            ShortcutAction::RecordingToggle => "Gravar/pausar a gravação",
        }
    }

    /// Acelerador padrão.
    pub fn default_accelerator(&self) -> &'static str {
        match self {
            ShortcutAction::ToggleOverlay => "CommandOrControl+Shift+O",
            ShortcutAction::ToggleSession => "CommandOrControl+Shift+S",
            ShortcutAction::ToggleStealth => "CommandOrControl+Shift+H",
            ShortcutAction::RequestInsight => "CommandOrControl+Shift+I",
            // Paridade com a barra do Perssua (⌘ Space / ⌥ C no macOS).
            ShortcutAction::FocusAsk => "CommandOrControl+Space",
            ShortcutAction::CaptureScreen => "Alt+C",
            ShortcutAction::RecordingToggle => "CommandOrControl+Shift+R",
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
            // O padrão usa CommandOrControl; a captura usa Alt (⌥C no Perssua).
            let accelerator = action.default_accelerator();
            assert!(
                accelerator.contains("CommandOrControl") || accelerator.contains("Alt+"),
                "acelerador inesperado para {:?}: {accelerator}",
                action
            );
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
