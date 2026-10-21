//! Porta de resolução do alvo de captura (janela ativa ou tela).
//!
//! Adaptador: `X11ScreenTargets`; dobra de teste: `FakeScreenTargets`.
//!
//! O Linux não tem API de exclusão de janela da captura (BR-006). Por isso o
//! alvo padrão é a **janela** da reunião: o overlay fica visível e fora do
//! vídeo (BR-070).

use crate::domain::recording::RecordingTarget;

/// Descobre o que capturar.
pub trait ScreenTargets: Send + Sync {
    /// Janela ativa (`_NET_ACTIVE_WINDOW`), se houver e não for o próprio app.
    fn active_window(&self) -> Option<RecordingTarget>;

    /// A tela inteira (geometria do display).
    fn screen(&self) -> Option<RecordingTarget>;
}
