//! Comandos IPC da janela **Perguntar** (a barra de pesquisa, BR-052).
//!
//! A barra é uma janela própria, aberta pela **lupa do notch** (disponível
//! independente de haver transcrição) e pelo atalho global.

use tauri::{Emitter, Manager};

use crate::error::{GhostWhisperError, Result};

/// Janela da barra "Perguntar".
pub const ASK_WINDOW: &str = "ask";

/// Largura lógica da barra de pesquisa (janela `ask`).
pub const ASK_WIDTH: f64 = 1040.0;

/// Distância da barra até o topo da tela (lógica).
const ASK_TOP_MARGIN: f64 = 12.0;

/// Centraliza a barra no **topo** do monitor principal.
///
/// A posição é recalculada a cada abertura: arrastar a barra vale enquanto ela
/// está aberta, mas ela sempre volta ao topo centralizado (BR-052) — foi o
/// jeito de ela nunca "nascer" fora da tela (escondendo a alça de arraste).
pub fn center_ask_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(ASK_WINDOW) else {
        return;
    };
    let Ok(Some(monitor)) = app.primary_monitor() else {
        return;
    };
    let size = monitor.size();
    let scale = monitor.scale_factor();
    let width = (ASK_WIDTH * scale).round() as i32;
    let margin = (ASK_TOP_MARGIN * scale).round() as i32;
    let x = (size.width as i32 - width) / 2;
    let _ = window.set_position(tauri::PhysicalPosition::new(x, margin));
}

/// Mostra a barra centralizada no topo e devolve o foco ao campo.
///
/// Se ela estava escondida, recentraliza **antes e depois** do `show`: o
/// gestor de janelas pode reencaixar a janela ao mapeá-la. O evento
/// `ask_focus` faz o campo receber o cursor (o atalho global precisa disso).
pub fn show_ask_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(ASK_WINDOW) else {
        return;
    };
    let was_hidden = !window.is_visible().unwrap_or(false);
    if was_hidden {
        center_ask_window(app);
    }
    let _ = window.show();
    if was_hidden {
        center_ask_window(app);
    }
    let _ = window.set_focus();
    let _ = app.emit_to(ASK_WINDOW, "ask_focus", ());
}

/// Abre (ou foca) a barra de pesquisa.
#[tauri::command]
pub fn ask_open(app: tauri::AppHandle) -> Result<()> {
    if app.get_webview_window(ASK_WINDOW).is_none() {
        return Err(GhostWhisperError::Other(
            "janela 'ask' não encontrada".to_string(),
        ));
    }
    show_ask_window(&app);
    Ok(())
}

/// Fecha a barra de pesquisa.
#[tauri::command]
pub fn ask_hide(app: tauri::AppHandle) -> Result<()> {
    let window = app
        .get_webview_window(ASK_WINDOW)
        .ok_or_else(|| GhostWhisperError::Other("janela 'ask' não encontrada".to_string()))?;
    let _ = window.hide();
    Ok(())
}

/// A barra está visível? (fallback quando o evento se perde)
#[tauri::command]
pub fn ask_is_open(app: tauri::AppHandle) -> Result<bool> {
    let window = app
        .get_webview_window(ASK_WINDOW)
        .ok_or_else(|| GhostWhisperError::Other("janela 'ask' não encontrada".to_string()))?;
    Ok(window.is_visible().unwrap_or(false))
}

/// Foca o campo da barra (emitido pelo atalho global).
#[tauri::command]
pub fn ask_focus(app: tauri::AppHandle) -> Result<()> {
    let _ = app.emit_to(ASK_WINDOW, "ask_focus", ());
    Ok(())
}
