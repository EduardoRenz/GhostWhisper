//! Adaptadores de leitura do ponteiro.
//!
//! - `X11PointerSource`: consulta `XQueryPointer` (X11/XWayland).
//! - `NullPointerSource`: fallback quando não há servidor X (Wayland puro),
//!   desativando o click-through automático.

use crate::domain::geometry::Point;
use crate::ports::pointer_source::PointerSource;

/// Ponteiro via X11.
pub struct X11PointerSource {
    conn: x11rb::rust_connection::RustConnection,
    root: u32,
}

impl X11PointerSource {
    /// Conecta ao servidor X e registra a janela raiz. `None` se indisponível.
    pub fn connect() -> Option<Self> {
        use x11rb::connection::Connection;

        let (conn, screen_num) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen_num)?.root;
        Some(Self { conn, root })
    }
}

impl PointerSource for X11PointerSource {
    fn position(&self) -> Option<Point> {
        use x11rb::protocol::xproto::ConnectionExt;

        let reply = self.conn.query_pointer(self.root).ok()?.reply().ok()?;
        Some(Point {
            x: reply.root_x as i32,
            y: reply.root_y as i32,
        })
    }
}

/// Ponteiro indisponível (retorna sempre `None`).
pub struct NullPointerSource;

impl PointerSource for NullPointerSource {
    fn position(&self) -> Option<Point> {
        None
    }
}
