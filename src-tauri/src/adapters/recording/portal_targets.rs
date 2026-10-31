//! Alvos de gravação no Wayland.
//!
//! O compositor não expõe a janela ativa nem a lista de janelas nativas ao
//! cliente XWayland. O seletor real é o **diálogo do portal ScreenCast**; aqui
//! publicamos um alvo único que representa "escolher no GNOME" e usa a
//! geometria do monitor primário apenas como metadado.

use crate::adapters::screen::x11_monitors;
use crate::domain::recording::RecordingTarget;
use crate::ports::screen_targets::ScreenTargets;

/// Alvos do portal: uma entrada para o seletor do GNOME.
pub struct PortalScreenTargets;

impl PortalScreenTargets {
    /// Cria o resolvedor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PortalScreenTargets {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenTargets for PortalScreenTargets {
    fn active_window(&self) -> Option<RecordingTarget> {
        // Janelas nativas não são visíveis pelo XWayland; quem escolhe é o portal.
        None
    }

    fn screen(&self) -> Option<RecordingTarget> {
        let monitors = x11_monitors::monitors();
        let monitor = monitors
            .iter()
            .find(|monitor| monitor.primary)
            .or_else(|| monitors.first());
        let (width, height, x, y) = monitor
            .map(|monitor| {
                (
                    u32::from(monitor.width),
                    u32::from(monitor.height),
                    monitor.x,
                    monitor.y,
                )
            })
            .unwrap_or((0, 0, 0, 0));
        Some(RecordingTarget::Screen {
            label: "Tela (portal)".to_string(),
            width,
            height,
            x,
            y,
        })
    }

    fn windows(&self) -> Vec<RecordingTarget> {
        Vec::new()
    }

    fn monitors(&self) -> Vec<RecordingTarget> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_a_single_portal_target() {
        let targets = PortalScreenTargets::new();
        let screen = targets.screen().expect("alvo");
        assert_eq!(screen.label(), "Tela (portal)");
        assert!(targets.windows().is_empty());
        assert!(targets.monitors().is_empty());
        assert!(targets.active_window().is_none());
    }
}
