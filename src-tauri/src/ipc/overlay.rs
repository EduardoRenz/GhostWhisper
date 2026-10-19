//! Comandos IPC do overlay.

use serde::Deserialize;
use tauri::State;

use crate::domain::geometry::Rect;
use crate::error::{GhostWhisperError, Result};
use crate::ports::overlay_window::Surface;
use crate::state::AppState;

/// Superfície do overlay escolhida pela UI.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceDto {
    Actionbar,
    Hidden,
}

impl From<SurfaceDto> for Surface {
    fn from(value: SurfaceDto) -> Self {
        match value {
            SurfaceDto::Actionbar => Surface::ActionBar,
            SurfaceDto::Hidden => Surface::Hidden,
        }
    }
}

/// Região interativa informada pela UI (coordenadas CSS, relativas à janela).
#[derive(Debug, Deserialize)]
pub struct InteractiveRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<InteractiveRegion> for Rect {
    fn from(value: InteractiveRegion) -> Self {
        Rect {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

/// Alterna a visibilidade do overlay; retorna `true` se ficou visível.
#[tauri::command]
pub fn overlay_toggle(state: State<'_, AppState>) -> Result<bool> {
    Ok(state.overlay.toggle())
}

/// Liga/desliga o modo stealth (fora da barra de tarefas, sem título).
#[tauri::command]
pub fn overlay_set_stealth(state: State<'_, AppState>, enabled: bool) -> Result<()> {
    state.overlay.set_stealth(enabled);
    // Persiste para reaplicar no próximo boot.
    let mut settings = state.settings.load()?;
    if settings.stealth != enabled {
        settings.stealth = enabled;
        state.settings.save(&settings)?;
    }
    Ok(())
}

/// Liga/desliga a coordenação automática de click-through.
#[tauri::command]
pub fn overlay_set_click_through(state: State<'_, AppState>, enabled: bool) -> Result<()> {
    state.click_through.set_enabled(enabled);
    Ok(())
}

/// Ajusta a superfície da janela (barra/expandida/oculta).
#[tauri::command]
pub fn overlay_set_surface(state: State<'_, AppState>, surface: SurfaceDto) -> Result<()> {
    state.overlay.set_surface(surface.into());
    Ok(())
}

/// Inicia o arraste da janela pelo usuário (posição passa a ser preservada).
#[tauri::command]
pub fn overlay_start_drag(state: State<'_, AppState>, window: tauri::WebviewWindow) -> Result<()> {
    state.overlay.set_anchor_bottom(false);
    window
        .start_dragging()
        .map_err(|error| GhostWhisperError::Other(format!("falha ao arrastar: {error}")))
}

/// Atualiza as regiões interativas usadas pelo click-through.
#[tauri::command(async)]
pub fn overlay_set_interactive_regions(
    state: State<'_, AppState>,
    regions: Vec<InteractiveRegion>,
) -> Result<()> {
    state
        .click_through
        .set_interactive_regions(regions.into_iter().map(Rect::from).collect());
    Ok(())
}
