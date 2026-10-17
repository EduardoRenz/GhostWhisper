//! Porta de coordenação do click-through (hover sobre regiões interativas).

use crate::domain::geometry::Rect;

/// Recebe as regiões interativas da UI e decide quando repassar cliques.
///
/// Adaptador: `CursorClickThroughController`.
pub trait ClickThroughController: Send + Sync {
    /// Habilita/desabilita a coordenação automática.
    ///
    /// Quando desabilitado, a janela volta a ser totalmente interativa.
    fn set_enabled(&self, enabled: bool);
    /// Atualiza as regiões interativas (coordenadas CSS, relativas à janela).
    fn set_interactive_regions(&self, regions: Vec<Rect>);
}
