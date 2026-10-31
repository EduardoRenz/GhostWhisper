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

    /// Converte o id persistido (`focus_ask`) de volta para a ação.
    pub fn from_str(value: &str) -> Option<Self> {
        Self::all()
            .into_iter()
            .find(|action| action.as_str() == value)
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

/// Converte um acelerador do Tauri para o formato do portal
/// (`CTRL+SHIFT+c`, conforme a especificação de shortcuts do freedesktop).
///
/// Devolve `None` quando o acelerador não é mapeável; nesse caso o portal deixa
/// o usuário escolher a combinação no diálogo de aprovação.
pub fn portal_trigger(accelerator: &str) -> Option<String> {
    let mut parts: Vec<&str> = accelerator.split('+').map(str::trim).collect();
    let key = parts.pop()?;
    if key.is_empty() {
        return None;
    }

    let mut modifiers: Vec<&str> = Vec::new();
    for modifier in parts {
        let mapped = match modifier.to_ascii_lowercase().as_str() {
            "commandorcontrol" | "ctrl" | "control" => "CTRL",
            "shift" => "SHIFT",
            "alt" | "option" => "ALT",
            "super" | "meta" | "command" | "cmd" | "logo" => "LOGO",
            _ => return None,
        };
        if !modifiers.contains(&mapped) {
            modifiers.push(mapped);
        }
    }
    // A ordem canônica do spec é CTRL, ALT, SHIFT, LOGO; o portal não exige,
    // mas manter a mesma saída deixa os testes estáveis.
    modifiers.sort_by_key(|modifier| match *modifier {
        "CTRL" => 0,
        "ALT" => 1,
        "SHIFT" => 2,
        _ => 3,
    });

    let key = portal_key(key)?;
    modifiers.push(&key);
    Some(modifiers.join("+"))
}

/// Nome xkbcommon (sem `XKB_KEY_`) da tecla.
fn portal_key(key: &str) -> Option<String> {
    if key.chars().count() == 1 {
        let character = key.chars().next()?;
        if character.is_ascii_alphanumeric() {
            return Some(character.to_ascii_lowercase().to_string());
        }
    }
    let lower = key.to_ascii_lowercase();
    let mapped = match lower.as_str() {
        "space" => "space",
        "enter" | "return" => "Return",
        "esc" | "escape" => "Escape",
        "tab" => "Tab",
        "backspace" => "BackSpace",
        "delete" | "del" => "Delete",
        "insert" | "ins" => "Insert",
        "home" => "Home",
        "end" => "End",
        "pageup" | "pgup" => "Page_Up",
        "pagedown" | "pgdown" | "pgdn" => "Page_Down",
        "arrowup" | "up" => "Up",
        "arrowdown" | "down" => "Down",
        "arrowleft" | "left" => "Left",
        "arrowright" | "right" => "Right",
        "print" | "printscreen" => "Print",
        "pause" => "Pause",
        "menu" => "Menu",
        "plus" => "plus",
        "minus" => "minus",
        "comma" => "comma",
        "period" => "period",
        "slash" => "slash",
        "semicolon" => "semicolon",
        "apostrophe" => "apostrophe",
        "grave" => "grave",
        "bracketleft" => "bracketleft",
        "bracketright" => "bracketright",
        "backslash" => "backslash",
        _ => {
            if let Some(number) = lower.strip_prefix('f') {
                if number
                    .parse::<u8>()
                    .is_ok_and(|number| (1..=24).contains(&number))
                {
                    return Some(lower.to_ascii_uppercase());
                }
            }
            return None;
        }
    };
    Some(mapped.to_string())
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

    #[test]
    fn maps_accelerators_to_portal_triggers() {
        assert_eq!(
            portal_trigger("CommandOrControl+Shift+C").as_deref(),
            Some("CTRL+SHIFT+c")
        );
        assert_eq!(portal_trigger("Alt+C").as_deref(), Some("ALT+c"));
        assert_eq!(
            portal_trigger("CommandOrControl+Space").as_deref(),
            Some("CTRL+space")
        );
        assert_eq!(
            portal_trigger("Ctrl+Alt+Delete").as_deref(),
            Some("CTRL+ALT+Delete")
        );
        assert_eq!(portal_trigger("F5").as_deref(), Some("F5"));
        assert_eq!(portal_trigger("Super+E").as_deref(), Some("LOGO+e"));
    }

    #[test]
    fn rejects_unknown_portal_keys() {
        assert_eq!(portal_trigger(""), None);
        assert_eq!(portal_trigger("CommandOrControl+Shift+Unknown"), None);
        assert_eq!(portal_trigger("Hyper+C"), None);
    }

    #[test]
    fn parses_actions_from_portal_ids() {
        for action in ShortcutAction::all() {
            assert_eq!(ShortcutAction::from_str(action.as_str()), Some(action));
        }
        assert_eq!(ShortcutAction::from_str("unknown"), None);
    }
}
