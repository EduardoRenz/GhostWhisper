//! Porta de captura de tela (BR-055).
//!
//! Adaptador: `X11ScreenCapture`; dobra: `FakeScreenCapture`.

use crate::error::Result;

/// Uma imagem capturada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedImage {
    /// Tipo MIME (ex.: `image/jpeg`).
    pub mime: String,
    /// Bytes da imagem (já comprimidos).
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Captura a tela.
///
/// O adaptador do Linux usa **X11/XWayland** (`x11rb`); em Wayland nativo não
/// há display X11 e a porta devolve **erro explícito** (BR-007) — o caminho
/// alternativo é colar a imagem (BR-055).
pub trait ScreenCapture: Send + Sync {
    /// Captura a tela e devolve a imagem comprimida.
    fn capture(&self) -> Result<CapturedImage>;
}
