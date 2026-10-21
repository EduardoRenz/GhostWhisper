//! Alvo de captura no X11: janela ativa (`_NET_ACTIVE_WINDOW`) ou tela (BR-070).
//!
//! O Linux não tem API de exclusão de janela da captura (BR-006); capturar a
//! janela é o que mantém o overlay visível e fora do vídeo.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, MapState, Window};
use x11rb::rust_connection::RustConnection;

use crate::domain::recording::RecordingTarget;
use crate::ports::screen_targets::ScreenTargets;

/// Descobre a janela ativa e a geometria da tela.
pub struct X11ScreenTargets;

impl X11ScreenTargets {
    /// Cria o resolvedor (nenhuma conexão é aberta aqui).
    pub fn new() -> Self {
        Self
    }
}

impl Default for X11ScreenTargets {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenTargets for X11ScreenTargets {
    fn active_window(&self) -> Option<RecordingTarget> {
        let (connection, screen_num) = RustConnection::connect(None).ok()?;
        let root = connection.setup().roots[screen_num].root;

        // A janela ativa é o caminho normal.
        if let Some(id) = active_window_id(&connection, root) {
            if let Some(target) = window_target(&connection, id) {
                return Some(target);
            }
        }

        // O overlay pode ter roubado o foco (ou o usuário clicou nele): cai
        // para a janela **do topo** que não é o GhostWhisper. Nunca a tela
        // inteira só porque o foco está no overlay (BR-070).
        topmost_window(&connection, root)
    }

    fn screen(&self) -> Option<RecordingTarget> {
        let (connection, screen_num) = RustConnection::connect(None).ok()?;
        let screen = &connection.setup().roots[screen_num];
        Some(RecordingTarget::Screen {
            label: "Tela inteira".to_string(),
            width: u32::from(screen.width_in_pixels),
            height: u32::from(screen.height_in_pixels),
        })
    }
}

/// Id da janela ativa (`_NET_ACTIVE_WINDOW`).
fn active_window_id(connection: &RustConnection, root: Window) -> Option<Window> {
    let atom = intern(connection, b"_NET_ACTIVE_WINDOW")?;
    let reply = connection
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    let id = reply.value32()?.next()?;
    (id != 0).then_some(id)
}

/// Janela do topo da pilha que não é o GhostWhisper.
fn topmost_window(connection: &RustConnection, root: Window) -> Option<RecordingTarget> {
    let atom = intern(connection, b"_NET_CLIENT_LIST_STACKING")?;
    let reply = connection
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1024)
        .ok()?
        .reply()
        .ok()?;
    let ids: Vec<Window> = reply.value32()?.collect();
    ids.iter()
        .rev()
        .find_map(|id| window_target(connection, *id))
}

/// Metadados e geometria de uma janela; `None` para o próprio app, janelas
/// ocultas, `override-redirect` ou vazias.
fn window_target(connection: &RustConnection, id: Window) -> Option<RecordingTarget> {
    let attributes = connection.get_window_attributes(id).ok()?.reply().ok()?;
    if attributes.map_state != MapState::VIEWABLE || attributes.override_redirect {
        return None;
    }
    let geometry = connection.get_geometry(id).ok()?.reply().ok()?;
    if geometry.width == 0 || geometry.height == 0 {
        return None;
    }

    let title = net_wm_name(connection, id).or_else(|| legacy_name(connection, id));
    let class = wm_class(connection, id);
    let label = label_from_parts(title.as_deref(), class.as_deref(), id);

    // O próprio GhostWhisper nunca é alvo: a gravação é da reunião.
    if label.to_lowercase().contains("ghostwhisper") {
        return None;
    }

    Some(RecordingTarget::Window {
        id,
        label,
        width: u32::from(geometry.width),
        height: u32::from(geometry.height),
    })
}

/// Interna um átomo (falha em vez de criar).
fn intern(connection: &RustConnection, name: &[u8]) -> Option<u32> {
    Some(connection.intern_atom(false, name).ok()?.reply().ok()?.atom)
}

/// Lê uma propriedade de texto.
fn property_string(
    connection: &RustConnection,
    window: Window,
    property: u32,
    kind: u32,
) -> Option<String> {
    let reply = connection
        .get_property(false, window, property, kind, 0, 1024)
        .ok()?
        .reply()
        .ok()?;
    if reply.value.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(&reply.value)
        .trim_matches('\0')
        .trim()
        .to_string();
    (!text.is_empty()).then_some(text)
}

/// `_NET_WM_NAME` (UTF-8).
fn net_wm_name(connection: &RustConnection, window: Window) -> Option<String> {
    let property = intern(connection, b"_NET_WM_NAME")?;
    let utf8 = intern(connection, b"UTF8_STRING")?;
    property_string(connection, window, property, utf8)
}

/// `WM_NAME` legado (Latin-1/STRING).
fn legacy_name(connection: &RustConnection, window: Window) -> Option<String> {
    let property = intern(connection, b"WM_NAME")?;
    property_string(connection, window, property, u32::from(AtomEnum::STRING))
}

/// `WM_CLASS` (instância + classe, separados por NUL).
fn wm_class(connection: &RustConnection, window: Window) -> Option<String> {
    let property = intern(connection, b"WM_CLASS")?;
    let kind = intern(connection, b"STRING")?;
    let reply = connection
        .get_property(false, window, property, kind, 0, 1024)
        .ok()?
        .reply()
        .ok()?;
    parse_wm_class(&reply.value)
}

/// Extrai a classe do `WM_CLASS` (`\0instância\0Classe\0`).
fn parse_wm_class(raw: &[u8]) -> Option<String> {
    let mut parts = raw
        .split(|byte| *byte == 0)
        .map(|part| String::from_utf8_lossy(part).trim().to_string())
        .filter(|part| !part.is_empty());
    let _instance = parts.next()?;
    parts.next().or(Some(_instance))
}

/// Rótulo legível: título — classe (ou um fallback pelo id).
fn label_from_parts(title: Option<&str>, class: Option<&str>, id: u32) -> String {
    match (title, class) {
        (Some(title), Some(class)) => format!("{title} — {class}"),
        (Some(title), None) => title.to_string(),
        (None, Some(class)) => class.to_string(),
        (None, None) => format!("Janela 0x{id:x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wm_class_and_prefers_the_class() {
        let raw = b"code\0Code\0";
        assert_eq!(parse_wm_class(raw), Some("Code".to_string()));
    }

    #[test]
    fn falls_back_to_the_instance_when_class_is_missing() {
        let raw = b"google-chrome\0";
        assert_eq!(parse_wm_class(raw), Some("google-chrome".to_string()));
    }

    #[test]
    fn empty_wm_class_is_none() {
        assert_eq!(parse_wm_class(b"\0\0"), None);
    }

    #[test]
    fn label_prefers_title_and_class() {
        assert_eq!(
            label_from_parts(Some("Reunião"), Some("Chrome"), 1),
            "Reunião — Chrome"
        );
        assert_eq!(label_from_parts(Some("Reunião"), None, 1), "Reunião");
        assert_eq!(label_from_parts(None, Some("Chrome"), 1), "Chrome");
        assert_eq!(label_from_parts(None, None, 0x2a), "Janela 0x2a");
    }
}
