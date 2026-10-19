//! IPC da janela de confirmação do drill-hover (BR-036).

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

use crate::domain::geometry::Rect;
use crate::error::{GhostWhisperError, Result};
use crate::state::AppState;

/// Retângulo do gatilho em CSS px, relativo à janela do overlay.
#[derive(Debug, Deserialize)]
pub struct DwellAnchor {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<DwellAnchor> for Rect {
    fn from(value: DwellAnchor) -> Self {
        Rect {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

/// Evento devolvido ao overlay quando a confirmação fecha.
const CLOSED_EVENT: &str = "dwell_closed";

#[derive(Clone, serde::Serialize)]
struct DwellClosed {
    confirmed: bool,
}

#[derive(Clone, serde::Serialize)]
struct DwellTrigger {
    entered: bool,
}

/// Abre a confirmação ancorada no gatilho.
#[tauri::command]
pub fn dwell_show(state: State<'_, AppState>, anchor: DwellAnchor, dwell_ms: u32) -> Result<()> {
    state.dwell.show(anchor.into(), dwell_ms)
}

/// Fecha a confirmação sem resultado (reset/cancelamento).
#[tauri::command]
pub fn dwell_hide(state: State<'_, AppState>) -> Result<()> {
    state.dwell.hide()
}

/// Duração do segundo estágio para a janela `dwell`.
#[tauri::command]
pub fn dwell_context(state: State<'_, AppState>) -> Option<u32> {
    state.dwell.context()
}

/// Resultado da confirmação: esconde a janela e avisa o overlay.
#[tauri::command]
pub fn dwell_result(app: AppHandle, state: State<'_, AppState>, confirmed: bool) -> Result<()> {
    state.dwell.hide()?;
    app.emit_to("overlay", CLOSED_EVENT, DwellClosed { confirmed })
        .map_err(|e| GhostWhisperError::Other(format!("evento do drill-hover: {e}")))
}

/// Informa à janela `dwell` se o cursor está no gatilho (para o grace local).
#[tauri::command]
pub fn dwell_trigger_hover(app: AppHandle, entered: bool) -> Result<()> {
    app.emit_to("dwell", "dwell_trigger", DwellTrigger { entered })
        .map_err(|e| GhostWhisperError::Other(format!("hover do drill-hover: {e}")))
}

/// Fecha a confirmação sem executar e reseta o overlay (usado quando o overlay
/// é ocultado por atalho/tray ou a superfície vira `hidden`).
pub(crate) fn dismiss(app: &AppHandle, state: &AppState) {
    let _ = state.dwell.hide();
    let _ = app.emit_to("overlay", CLOSED_EVENT, DwellClosed { confirmed: false });
}
