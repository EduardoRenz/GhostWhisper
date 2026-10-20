//! Comandos IPC da janela **Perguntar** (a barra de pesquisa, BR-052).
//!
//! A barra é uma janela própria, aberta pela **lupa do notch** (disponível
//! independente de haver transcrição) e pelo atalho global.

use tauri::{Emitter, Manager};

use crate::error::{GhostWhisperError, Result};

/// Janela da barra "Perguntar".
pub const ASK_WINDOW: &str = "ask";

/// Abre (ou foca) a barra de pesquisa.
#[tauri::command]
pub fn ask_open(app: tauri::AppHandle) -> Result<()> {
    let window = app
        .get_webview_window(ASK_WINDOW)
        .ok_or_else(|| GhostWhisperError::Other("janela 'ask' não encontrada".to_string()))?;
    window
        .show()
        .map_err(|error| GhostWhisperError::Other(error.to_string()))?;
    window
        .set_focus()
        .map_err(|error| GhostWhisperError::Other(error.to_string()))?;
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
