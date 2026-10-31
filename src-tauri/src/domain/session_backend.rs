//! Sessão gráfica do Linux (X11 ou Wayland).
//!
//! O GhostWhisper usa caminhos diferentes por sessão: X11 para o overlay/atalhos
//! nativos e portais do XDG no Wayland. A decisão é pura (sem ler o ambiente) e
//! testável; a leitura das variáveis fica no composition root.

/// Backend gráfico da sessão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionBackend {
    /// X11 ou XWayland com servidor X acessível.
    X11,
    /// Sessão Wayland nativa.
    Wayland,
}

impl SessionBackend {
    /// Decide a partir de `XDG_SESSION_TYPE` e `WAYLAND_DISPLAY`.
    ///
    /// Qualquer um dos dois indicando Wayland já basta: em XWayland o
    /// `WAYLAND_DISPLAY` continua definido, e os recursos globais continuam
    /// limitados pelo compositor.
    pub fn detect(session_type: Option<&str>, wayland_display: Option<&str>) -> Self {
        let wayland_display = wayland_display.is_some_and(|value| !value.trim().is_empty());
        let wayland_session =
            session_type.is_some_and(|value| value.trim().eq_ignore_ascii_case("wayland"));
        if wayland_display || wayland_session {
            Self::Wayland
        } else {
            Self::X11
        }
    }

    /// `true` quando é uma sessão Wayland (nativa ou XWayland).
    pub fn is_wayland(self) -> bool {
        matches!(self, Self::Wayland)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wayland_by_session_type() {
        assert_eq!(
            SessionBackend::detect(Some("wayland"), None),
            SessionBackend::Wayland
        );
        assert_eq!(
            SessionBackend::detect(Some("WAYLAND"), None),
            SessionBackend::Wayland
        );
    }

    #[test]
    fn detects_wayland_by_display() {
        assert_eq!(
            SessionBackend::detect(None, Some("wayland-0")),
            SessionBackend::Wayland
        );
    }

    #[test]
    fn empty_or_x11_is_x11() {
        assert_eq!(SessionBackend::detect(None, None), SessionBackend::X11);
        assert_eq!(
            SessionBackend::detect(Some("x11"), Some("")),
            SessionBackend::X11
        );
        assert_eq!(
            SessionBackend::detect(Some("tty"), Some("  ")),
            SessionBackend::X11
        );
    }

    #[test]
    fn wayland_predicate() {
        assert!(SessionBackend::Wayland.is_wayland());
        assert!(!SessionBackend::X11.is_wayland());
    }
}
