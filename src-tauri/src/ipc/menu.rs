//! Comandos IPC da janela flutuante de menus (agentes/canais).

use serde::Deserialize;
use tauri::State;

use crate::ports::menu_window::{MenuAnchor, MenuKind, MenuRect};
use crate::state::AppState;

/// Tipo de menu pedido pela UI.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MenuKindDto {
    Agent,
    Channel,
}

impl From<MenuKindDto> for MenuKind {
    fn from(value: MenuKindDto) -> Self {
        match value {
            MenuKindDto::Agent => MenuKind::Agent,
            MenuKindDto::Channel => MenuKind::Channel,
        }
    }
}

/// Retângulo em CSS px relativo à janela do overlay.
#[derive(Debug, Deserialize)]
pub struct MenuRectDto {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<MenuRectDto> for MenuRect {
    fn from(value: MenuRectDto) -> Self {
        MenuRect {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

/// Gatilho (eixo X) e corpo do notch (eixo Y) do menu.
#[derive(Debug, Deserialize)]
pub struct MenuAnchorDto {
    pub trigger: MenuRectDto,
    pub notch: MenuRectDto,
}

impl From<MenuAnchorDto> for MenuAnchor {
    fn from(value: MenuAnchorDto) -> Self {
        MenuAnchor {
            trigger: value.trigger.into(),
            notch: value.notch.into(),
        }
    }
}

/// Abre (ou reposiciona) o menu ancorado no gatilho.
#[tauri::command]
pub fn menu_open(state: State<'_, AppState>, kind: MenuKindDto, anchor: MenuAnchorDto) {
    state.menu.open(kind.into(), anchor.into());
}

/// Ajusta a altura do menu ao conteúdo medido (px lógicos).
#[tauri::command]
pub fn menu_resize(state: State<'_, AppState>, height: f64) {
    state.menu.resize(height);
}

/// Fecha o menu.
#[tauri::command]
pub fn menu_close(state: State<'_, AppState>) {
    state.menu.close();
}

/// Menu atualmente aberto (`agent`/`channel`) — estado inicial do webview.
#[tauri::command]
pub fn menu_state(state: State<'_, AppState>) -> Option<&'static str> {
    state.menu.current().map(|kind| match kind {
        MenuKind::Agent => "agent",
        MenuKind::Channel => "channel",
    })
}
