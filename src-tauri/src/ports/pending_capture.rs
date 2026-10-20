//! Porta do **print pendente** (anexado à pergunta, BR-055).
//!
//! A captura vive em memória entre as janelas: a barra de pesquisa e a
//! **prévia** no canto da tela leem o mesmo print. Ele só é enviado quando o
//! usuário confirma a pergunta.
//!
//! Adaptador: `InMemoryPendingCapture`; dobra: o próprio (em memória).

use crate::error::Result;
use crate::ports::screen_capture::CapturedImage;

/// Guarda o print capturado e ainda **não enviado**.
pub trait PendingCapture: Send + Sync {
    /// Guarda (ou substitui) o print pendente.
    fn set(&self, image: CapturedImage) -> Result<()>;
    /// Lê o print pendente (sem removê-lo).
    fn get(&self) -> Result<Option<CapturedImage>>;
    /// Descarta o print pendente (botão de remover).
    fn clear(&self) -> Result<()>;
}
