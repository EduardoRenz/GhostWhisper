//! Adaptador da janela overlay sobre `tauri::WebviewWindow`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{LogicalSize, PhysicalPosition, WebviewWindow};

use crate::domain::geometry::Point;
use crate::ports::overlay_window::{OverlayWindowController, Surface};

/// Dimensões lógicas por superfície `(largura, altura)`.
const ACTIONBAR_SIZE: (f64, f64) = (900.0, 72.0);
const EXPANDED_SIZE: (f64, f64) = (900.0, 380.0);

/// Implementa `OverlayWindowController` sobre a janela Tauri.
pub struct TauriOverlayWindow {
    window: WebviewWindow,
    click_through: Arc<AtomicBool>,
    anchor_bottom: AtomicBool,
}

impl TauriOverlayWindow {
    /// Cria o adaptador a partir de uma janela Tauri existente.
    pub fn new(window: WebviewWindow) -> Self {
        Self {
            window,
            click_through: Arc::new(AtomicBool::new(false)),
            anchor_bottom: AtomicBool::new(true),
        }
    }
}

impl OverlayWindowController for TauriOverlayWindow {
    fn show(&self) {
        let _ = self.window.show();
    }

    fn hide(&self) {
        let _ = self.window.hide();
    }

    fn toggle(&self) -> bool {
        if self.is_visible() {
            self.hide();
            false
        } else {
            self.show();
            true
        }
    }

    fn is_visible(&self) -> bool {
        self.window.is_visible().unwrap_or(false)
    }

    fn set_click_through(&self, enabled: bool) {
        // Evita chamadas redundantes ao compositor.
        if self.click_through.swap(enabled, Ordering::Relaxed) == enabled {
            return;
        }
        let _ = self.window.set_ignore_cursor_events(enabled);
    }

    fn set_stealth(&self, enabled: bool) {
        let _ = self.window.set_skip_taskbar(enabled);
        let _ = self
            .window
            .set_title(if enabled { "" } else { "GhostWhisper" });
    }

    fn set_surface(&self, surface: Surface) {
        let (width, height) = match surface {
            Surface::ActionBar => ACTIONBAR_SIZE,
            Surface::Expanded => EXPANDED_SIZE,
            Surface::Hidden => {
                self.hide();
                return;
            }
        };

        let previous = self.window.outer_size().ok();
        let position = self.window.outer_position().ok();
        let _ = self.window.set_size(LogicalSize::new(width, height));

        // Reancora pela base apenas enquanto o usuário não moveu a janela.
        if self.anchor_bottom.load(Ordering::Relaxed) {
            if let (Some(previous), Some(position)) = (previous, position) {
                let scale = self.window.scale_factor().unwrap_or(1.0);
                let new_height = (height * scale).round() as i32;
                let delta = previous.height as i32 - new_height;
                let _ = self
                    .window
                    .set_position(PhysicalPosition::new(position.x, position.y + delta));
            }
        }
    }

    fn set_anchor_bottom(&self, anchored: bool) {
        self.anchor_bottom.store(anchored, Ordering::Relaxed);
    }

    fn anchor_bottom(&self) -> bool {
        self.anchor_bottom.load(Ordering::Relaxed)
    }

    fn outer_position(&self) -> Option<Point> {
        self.window
            .outer_position()
            .ok()
            .map(|p| Point { x: p.x, y: p.y })
    }

    fn outer_size(&self) -> Option<(i32, i32)> {
        self.window
            .outer_size()
            .ok()
            .map(|s| (s.width as i32, s.height as i32))
    }

    fn scale_factor(&self) -> f64 {
        self.window.scale_factor().unwrap_or(1.0)
    }
}
