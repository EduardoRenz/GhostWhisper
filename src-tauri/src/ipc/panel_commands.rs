//! Comandos IPC das janelas de painel (resumo, sugestões, chat e histórico).
//!
//! As janelas são declaradas ocultas no `tauri.conf.json`; abrir/fechar aqui
//! mantém a UI informada pelos eventos `panel_opened` / `panel_hidden`.

use tauri::{Emitter, Manager};

use crate::domain::settings::PANEL_LABELS;
use crate::error::{GhostWhisperError, Result};

/// Evento emitido quando um painel fica visível.
pub const PANEL_OPENED_EVENT: &str = "panel_opened";
/// Evento emitido quando um painel é escondido.
pub const PANEL_HIDDEN_EVENT: &str = "panel_hidden";

/// Valida e devolve o label do painel.
fn resolve_label(label: &str) -> Result<String> {
    let normalized = label.trim().to_lowercase();
    if PANEL_LABELS.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(GhostWhisperError::Config(format!(
            "painel desconhecido: {label}"
        )))
    }
}

/// Abre (ou foca) uma janela de painel e avisa a UI.
#[tauri::command]
pub fn panel_open(app: tauri::AppHandle, label: String) -> Result<()> {
    let label = resolve_label(&label)?;
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| GhostWhisperError::Other(format!("janela '{label}' não encontrada")))?;
    let _ = window.unminimize();
    window
        .show()
        .map_err(|error| GhostWhisperError::Other(error.to_string()))?;
    window
        .set_focus()
        .map_err(|error| GhostWhisperError::Other(error.to_string()))?;
    let payload = label.clone();
    let _ = app.emit_to(label.as_str(), PANEL_OPENED_EVENT, payload);
    Ok(())
}

/// Esconde uma janela de painel e avisa a UI.
#[tauri::command]
pub fn panel_hide(app: tauri::AppHandle, label: String) -> Result<()> {
    let label = resolve_label(&label)?;
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| GhostWhisperError::Other(format!("janela '{label}' não encontrada")))?;
    let _ = window.hide();
    let payload = label.clone();
    let _ = app.emit_to(label.as_str(), PANEL_HIDDEN_EVENT, payload);
    Ok(())
}

/// Inicia o arraste da janela chamadora.
#[tauri::command]
pub fn panel_start_drag(window: tauri::WebviewWindow) -> Result<()> {
    window
        .start_dragging()
        .map_err(|error| GhostWhisperError::Other(format!("falha ao arrastar: {error}")))
}

/// Informa se o painel está visível (fallback quando o evento se perde).
#[tauri::command]
pub fn panel_is_open(app: tauri::AppHandle, label: String) -> Result<bool> {
    let label = resolve_label(&label)?;
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| GhostWhisperError::Other(format!("janela '{label}' não encontrada")))?;
    Ok(window.is_visible().unwrap_or(false))
}
