//! Porta da janela de confirmação do drill-hover (BR-036).
//!
//! A janela é separada do notch para que o popover flutue acima/abaixo do botão
//! sem redimensionar a barra (uma `position:absolute` não sai da janela nativa).

use crate::domain::geometry::Rect;
use crate::error::Result;

/// Controla a janela `dwell` (mostrar, esconder e contexto de duração).
pub trait DwellPopoverController: Send + Sync {
    /// Abre a confirmação ancorada no retângulo do gatilho (CSS px, relativo à
    /// janela do overlay). O adaptador decide acima/abaixo pela área útil.
    fn show(&self, anchor: Rect, dwell_ms: u32) -> Result<()>;
    /// Esconde a confirmação (cancelamento/reset).
    fn hide(&self) -> Result<()>;
    /// Duração atual do estágio de confirmação (para a janela `dwell`).
    fn context(&self) -> Option<u32>;
}
