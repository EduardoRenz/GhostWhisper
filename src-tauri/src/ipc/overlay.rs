//! Comandos IPC do overlay.

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

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
pub fn overlay_toggle(app: AppHandle, state: State<'_, AppState>) -> Result<bool> {
    let visible = state.overlay.toggle();
    if !visible {
        crate::ipc::dwell::dismiss(&app, &state);
    }
    Ok(visible)
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
pub fn overlay_set_surface(
    app: AppHandle,
    state: State<'_, AppState>,
    surface: SurfaceDto,
) -> Result<()> {
    let surface: Surface = surface.into();
    if matches!(surface, Surface::Hidden) {
        crate::ipc::dwell::dismiss(&app, &state);
    }
    state.overlay.set_surface(surface);
    Ok(())
}

/// Ajusta a largura lógica da barra (medida pela UI) — a janela acompanha o
/// conteúdo, sem sobra fixa.
#[tauri::command]
pub fn overlay_set_actionbar_width(state: State<'_, AppState>, width: f64) -> Result<()> {
    if !width.is_finite() || width <= 0.0 {
        return Err(GhostWhisperError::Config(format!(
            "largura da barra inválida: {width}"
        )));
    }
    state.overlay.set_actionbar_width(width);
    Ok(())
}

/// Reserva/libera altura extra abaixo da barra (menu de canais aberto).
#[tauri::command]
pub fn overlay_set_popover_extra_height(state: State<'_, AppState>, height: f64) -> Result<()> {
    if !height.is_finite() || height < 0.0 {
        return Err(GhostWhisperError::Config(format!(
            "altura extra inválida: {height}"
        )));
    }
    state.overlay.set_popover_extra_height(height);
    Ok(())
}

/// Inicia o arraste da janela pelo usuário (posição passa a ser preservada).
#[tauri::command]
pub fn overlay_start_drag(
    app: AppHandle,
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<()> {
    state.overlay.set_anchor_bottom(false);
    // Arrastar o notch significa "Flutuante Livre" (BR-062): a posição deixa de
    // ser ditada pela ancoragem e passa a ser a do usuário.
    if let Ok(mut settings) = state.settings.load() {
        if settings.notch_anchor != "free" {
            settings.notch_anchor = "free".to_string();
            let _ = state.settings.save(&settings);
            let _ = app.emit("settings_changed", ());
        }
        // Mantém o adaptador em sincronia: sem isso, o próximo `set_surface`
        // reaplicaria a âncora antiga e puxaria a janela de volta.
        let density = crate::domain::hud_layout::HudDensity::parse(&settings.hud_density);
        state
            .overlay
            .set_hud_layout(density, crate::domain::hud_layout::NotchAnchor::Free);
    }
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
