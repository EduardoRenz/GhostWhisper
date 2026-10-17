//! Porta de leitura da posição global do ponteiro.

use crate::domain::geometry::Point;

/// Fornece a posição global do cursor (pixels físicos).
///
/// Adaptador X11: `X11PointerSource`. Sob Wayland (ou sem servidor X) a
/// implementação retorna `None` e o click-through automático fica desativado.
pub trait PointerSource: Send + Sync {
    /// Posição atual do ponteiro, ou `None` se indisponível.
    fn position(&self) -> Option<Point>;
}
