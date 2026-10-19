//! Janela de confirmação do drill-hover sobre `tauri::WebviewWindow`.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, LogicalSize, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::domain::geometry::{dwell_popover_position, Rect};
use crate::error::{GhostWhisperError, Result};
use crate::ports::dwell_popover::DwellPopoverController;

/// Tamanho lógico da janela de confirmação (pill + sombra).
const POPOVER_WIDTH: f64 = 260.0;
const POPOVER_HEIGHT: f64 = 60.0;
/// Distância (px físicos) entre o gatilho e a popover.
const GAP: i32 = 8;
/// Margem de conforto (px físicos) da área útil.
const MARGIN: i32 = 8;

/// Evento com a duração do segundo estágio para a janela `dwell`.
const CONTEXT_EVENT: &str = "dwell_context";
/// Evento para a janela `dwell` resetar o timer local (fechou/cancelou).
const RESET_EVENT: &str = "dwell_reset";

#[derive(Clone, Serialize)]
struct DwellContext {
    dwell_ms: u32,
    /// Identifica a abertura; a janela usa para resetar o estado local.
    token: u64,
    /// Geometria física real da janela (para o poll do cursor).
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale: f64,
}

fn open_token() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Implementa `DwellPopoverController` gerenciando a janela `dwell`.
pub struct TauriDwellPopover {
    window: WebviewWindow,
    overlay: WebviewWindow,
    dwell_ms: Mutex<Option<u32>>,
}

impl TauriDwellPopover {
    /// Cria o adaptador com a janela de confirmação e a janela do notch.
    pub fn new(window: WebviewWindow, overlay: WebviewWindow) -> Self {
        Self {
            window,
            overlay,
            dwell_ms: Mutex::new(None),
        }
    }

    fn physical_rect(&self, anchor: Rect) -> Result<(Rect, Rect, (i32, i32), f64)> {
        let position = self.overlay.outer_position().map_err(|e| {
            GhostWhisperError::Other(format!("posição do overlay indisponível: {e}"))
        })?;
        let scale = self.overlay.scale_factor().unwrap_or(1.0);
        let monitor = self
            .overlay
            .current_monitor()
            .map_err(|e| GhostWhisperError::Other(format!("monitor indisponível: {e}")))?
            .ok_or_else(|| GhostWhisperError::Other("monitor do overlay ausente".to_string()))?;
        let work = monitor.work_area();

        let mut anchor_phys = anchor.scaled(scale);
        anchor_phys.x += position.x;
        anchor_phys.y += position.y;

        let work_area = Rect {
            x: work.position.x,
            y: work.position.y,
            width: work.size.width as i32,
            height: work.size.height as i32,
        };
        let size = (
            (POPOVER_WIDTH * scale).round() as i32,
            (POPOVER_HEIGHT * scale).round() as i32,
        );
        Ok((anchor_phys, work_area, size, scale))
    }
}

impl DwellPopoverController for TauriDwellPopover {
    fn show(&self, anchor: Rect, dwell_ms: u32) -> Result<()> {
        let (anchor, work_area, size, scale) = self.physical_rect(anchor)?;
        let layout = dwell_popover_position(
            anchor,
            work_area,
            size,
            (GAP as f64 * scale).round() as i32,
            (MARGIN as f64 * scale).round() as i32,
        );

        self.window
            .set_size(LogicalSize::new(POPOVER_WIDTH, POPOVER_HEIGHT))
            .map_err(|e| GhostWhisperError::Other(format!("tamanho do popover: {e}")))?;
        self.window
            .set_position(PhysicalPosition::new(layout.x, layout.y))
            .map_err(|e| GhostWhisperError::Other(format!("posição do popover: {e}")))?;

        if let Ok(mut current) = self.dwell_ms.lock() {
            *current = Some(dwell_ms);
        }
        self.window
            .show()
            .map_err(|e| GhostWhisperError::Other(format!("mostrar popover: {e}")))?;
        // Geometria **real** após mostrar: o poll do cursor usa estas coordenadas
        // em vez de consultar o webview (que pode estar defasado na 2ª abertura).
        let position = self
            .window
            .outer_position()
            .unwrap_or(PhysicalPosition::new(layout.x, layout.y));
        let actual_size = self.window.outer_size().unwrap_or(PhysicalSize::new(
            size.0.max(1) as u32,
            size.1.max(1) as u32,
        ));
        let actual_scale = self.window.scale_factor().unwrap_or(scale);
        if let Err(error) = self.window.emit(
            CONTEXT_EVENT,
            DwellContext {
                dwell_ms,
                token: open_token(),
                x: position.x,
                y: position.y,
                width: actual_size.width,
                height: actual_size.height,
                scale: actual_scale,
            },
        ) {
            // A janela já está visível; o contexto é só a duração/geometria.
            tracing::warn!("falha ao emitir contexto do drill-hover: {error}");
        }
        Ok(())
    }

    fn hide(&self) -> Result<()> {
        if let Ok(mut current) = self.dwell_ms.lock() {
            *current = None;
        }
        let _ = self.window.emit(RESET_EVENT, ());
        self.window
            .hide()
            .map_err(|e| GhostWhisperError::Other(format!("esconder popover: {e}")))?;
        Ok(())
    }

    fn context(&self) -> Option<u32> {
        self.dwell_ms.lock().ok().and_then(|current| *current)
    }
}
