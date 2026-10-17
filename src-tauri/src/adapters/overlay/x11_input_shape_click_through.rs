//! Click-through definitivo no X11 via *input shape* por região.
//!
//! Em vez de alternar globalmente o repasse de cliques (o que exige polling do
//! cursor e tem corrida), definimos a **região de entrada** da janela como a
//! união dos retângulos interativos. Fora deles o clique atravessa; dentro, a
//! janela é interativa de imediato — sem polling e sem atraso.
//!
//! Fallback (Wayland/sem X11): `CursorClickThroughController`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::shape::{ConnectionExt as ShapeConnectionExt, SK, SO};
use x11rb::protocol::xproto::{ClipOrdering, Rectangle};
use x11rb::rust_connection::RustConnection;

use crate::domain::geometry::Rect;
use crate::ports::click_through::ClickThroughController;
use crate::ports::overlay_window::OverlayWindowController;

/// Aplica o click-through por região via extensão SHAPE do X11.
pub struct X11InputShapeClickThrough {
    conn: RustConnection,
    window: u32,
    overlay: Arc<dyn OverlayWindowController>,
    regions: Mutex<Vec<Rect>>,
    enabled: AtomicBool,
}

impl X11InputShapeClickThrough {
    /// Conecta ao X e obtém o `xid` da janela. `None` se não for X11/SHAPE.
    pub fn connect(
        window: &tauri::WebviewWindow,
        overlay: Arc<dyn OverlayWindowController>,
    ) -> Option<Self> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let xid = match window.window_handle().ok()?.as_raw() {
            RawWindowHandle::Xlib(handle) => handle.window as u32,
            _ => return None,
        };

        let (conn, _screen) = x11rb::connect(None).ok()?;
        // Sem a extensão SHAPE não há como fazer input-shape por região.
        conn.extension_information(x11rb::protocol::shape::X11_EXTENSION_NAME)
            .ok()??;

        Some(Self {
            conn,
            window: xid,
            overlay,
            regions: Mutex::new(Vec::new()),
            enabled: AtomicBool::new(true),
        })
    }

    /// (Re)aplica a região de entrada com base nas regiões interativas atuais.
    pub fn apply(&self) {
        let enabled = self.enabled.load(Ordering::Relaxed);
        let regions = self.regions.lock().map(|r| r.clone()).unwrap_or_default();

        let rectangles: Vec<Rectangle> = if !enabled || regions.is_empty() {
            // Janela inteira interativa (evita "overlay morto" antes da UI publicar).
            let size = self.overlay.outer_size().unwrap_or((1, 1));
            vec![Rectangle {
                x: 0,
                y: 0,
                width: size.0.max(1).min(u16::MAX as i32) as u16,
                height: size.1.max(1).min(u16::MAX as i32) as u16,
            }]
        } else {
            let scale = self.overlay.scale_factor();
            regions
                .iter()
                .map(|rect| rect.scaled(scale))
                .filter(|rect| rect.width > 0 && rect.height > 0)
                .map(|rect| Rectangle {
                    x: rect.x.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                    y: rect.y.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                    width: rect.width.min(u16::MAX as i32) as u16,
                    height: rect.height.min(u16::MAX as i32) as u16,
                })
                .collect()
        };

        if let Err(error) = self.conn.shape_rectangles(
            SO::SET,
            SK::INPUT,
            ClipOrdering::UNSORTED,
            self.window,
            0,
            0,
            &rectangles,
        ) {
            tracing::warn!("falha ao aplicar input shape: {error}");
            return;
        }
        let _ = self.conn.flush();
    }
}

impl ClickThroughController for X11InputShapeClickThrough {
    fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        self.apply();
    }

    fn set_interactive_regions(&self, regions: Vec<Rect>) {
        if let Ok(mut guard) = self.regions.lock() {
            *guard = regions;
        }
        self.apply();
    }
}
