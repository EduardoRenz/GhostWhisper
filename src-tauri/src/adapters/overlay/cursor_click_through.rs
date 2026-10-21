//! Coordenador de click-through por posição do cursor.
//!
//! Como o protocolo gráfico do Linux não permite "furar" o clique por região,
//! alternamos `set_ignore_cursor_events` globalmente conforme o ponteiro entra
//! ou sai das regiões interativas informadas pela UI.
//!
//! Para evitar cliques perdidos (ISS-001), a entrada numa região é imediata e a
//! saída é debounced (`ClickThroughLatch`), além de uma margem de conforto em
//! torno das regiões.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::domain::geometry::{should_pass_through_margin, ClickThroughLatch, Rect};
use crate::ports::click_through::ClickThroughController;
use crate::ports::overlay_window::OverlayWindowController;
use crate::ports::pointer_source::PointerSource;

/// Intervalo de amostragem do cursor.
const POLL_INTERVAL: Duration = Duration::from_millis(30);
/// Margem de conforto (px) em torno das regiões interativas.
const ENTER_MARGIN_PX: i32 = 8;
/// Amostras consecutivas fora das regiões antes de reativar o click-through.
const LEAVE_SAMPLES: u32 = 3;

/// Coordena o click-through automático.
pub struct CursorClickThroughController {
    overlay: Arc<dyn OverlayWindowController>,
    pointer: Arc<dyn PointerSource>,
    regions: Mutex<Vec<Rect>>,
    latch: Mutex<ClickThroughLatch>,
    enabled: AtomicBool,
    active: AtomicBool,
}

impl CursorClickThroughController {
    /// Cria o coordenador com as portas de janela e de ponteiro.
    pub fn new(overlay: Arc<dyn OverlayWindowController>, pointer: Arc<dyn PointerSource>) -> Self {
        Self {
            overlay,
            pointer,
            regions: Mutex::new(Vec::new()),
            latch: Mutex::new(ClickThroughLatch::new(LEAVE_SAMPLES)),
            enabled: AtomicBool::new(true),
            active: AtomicBool::new(false),
        }
    }

    /// Inicia a thread de amostragem do cursor.
    pub fn spawn(self: &Arc<Self>) {
        let this = Arc::clone(self);
        thread::spawn(move || loop {
            thread::sleep(POLL_INTERVAL);
            this.tick();
        });
    }

    /// Uma amostra: lê o cursor e aplica o estado desejado de click-through.
    fn tick(&self) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let Some(position) = self.pointer.position() else {
            return;
        };
        let Some(window_position) = self.overlay.outer_position() else {
            return;
        };
        let Some(window_size) = self.overlay.outer_size() else {
            return;
        };
        let scale = self.overlay.scale_factor();
        let regions = self.regions.lock().map(|r| r.clone()).unwrap_or_default();

        let desired = should_pass_through_margin(
            position,
            window_position,
            window_size,
            scale,
            &regions,
            ENTER_MARGIN_PX,
        );
        let pass = self
            .latch
            .lock()
            .map(|mut latch| latch.update(desired))
            .unwrap_or(desired);

        if self.active.load(Ordering::Relaxed) != pass {
            self.overlay.set_click_through(pass);
            self.active.store(pass, Ordering::Relaxed);
        }
    }

    fn reset_latch(&self) {
        if let Ok(mut latch) = self.latch.lock() {
            latch.reset();
        }
    }
}

impl ClickThroughController for CursorClickThroughController {
    fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        self.reset_latch();
        if !enabled {
            self.active.store(false, Ordering::Relaxed);
            self.overlay.set_click_through(false);
        }
    }

    fn set_interactive_regions(&self, regions: Vec<Rect>) {
        let empty = regions.is_empty();
        if let Ok(mut guard) = self.regions.lock() {
            *guard = regions;
        }
        // Sem regiões a janela é totalmente interativa: zera a histerese.
        if empty {
            self.reset_latch();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::geometry::Point;
    use crate::ports::overlay_window::Surface;

    struct FakeOverlay {
        position: Point,
        size: (i32, i32),
        calls: Mutex<Vec<bool>>,
    }
    impl FakeOverlay {
        fn calls(&self) -> Vec<bool> {
            self.calls.lock().unwrap().clone()
        }
    }
    impl OverlayWindowController for FakeOverlay {
        fn show(&self) {}
        fn hide(&self) {}
        fn toggle(&self) -> bool {
            true
        }
        fn is_visible(&self) -> bool {
            true
        }
        fn set_click_through(&self, enabled: bool) {
            self.calls.lock().unwrap().push(enabled);
        }
        fn set_stealth(&self, _enabled: bool) {}
        fn set_surface(&self, _surface: Surface) {}
        fn set_hud_layout(
            &self,
            _density: crate::domain::hud_layout::HudDensity,
            _anchor: crate::domain::hud_layout::NotchAnchor,
        ) {
        }
        fn set_actionbar_width(&self, _width: f64) {}
        fn set_popover_extra_height(&self, _extra: f64) {}
        fn set_anchor_bottom(&self, _anchored: bool) {}
        fn anchor_bottom(&self) -> bool {
            true
        }
        fn outer_position(&self) -> Option<Point> {
            Some(self.position)
        }
        fn outer_size(&self) -> Option<(i32, i32)> {
            Some(self.size)
        }
        fn scale_factor(&self) -> f64 {
            1.0
        }
    }

    struct FakePointer {
        position: Mutex<Option<Point>>,
    }
    impl FakePointer {
        fn set(&self, x: i32, y: i32) {
            *self.position.lock().unwrap() = Some(Point { x, y });
        }
    }
    impl PointerSource for FakePointer {
        fn position(&self) -> Option<Point> {
            *self.position.lock().unwrap()
        }
    }

    fn setup() -> (
        Arc<CursorClickThroughController>,
        Arc<FakeOverlay>,
        Arc<FakePointer>,
    ) {
        let overlay = Arc::new(FakeOverlay {
            position: Point { x: 100, y: 100 },
            size: (200, 80),
            calls: Mutex::new(Vec::new()),
        });
        let pointer = Arc::new(FakePointer {
            position: Mutex::new(Some(Point { x: 0, y: 0 })),
        });
        let controller = Arc::new(CursorClickThroughController::new(
            overlay.clone(),
            pointer.clone(),
        ));
        (controller, overlay, pointer)
    }

    fn region() -> Rect {
        Rect {
            x: 10,
            y: 10,
            width: 50,
            height: 20,
        }
    }

    #[test]
    fn no_regions_keeps_window_interactive() {
        let (controller, overlay, pointer) = setup();
        pointer.set(0, 0);
        controller.tick();
        assert!(!overlay.calls().contains(&true));
    }

    #[test]
    fn enters_interactive_immediately_on_region() {
        let (controller, overlay, pointer) = setup();
        controller.set_interactive_regions(vec![region()]);
        // local (20,20): dentro da região.
        pointer.set(120, 120);
        controller.tick();
        // Já era interativo por padrão; nada de pass-through.
        assert!(!overlay.calls().contains(&true));
    }

    #[test]
    fn pass_through_after_debounce() {
        let (controller, overlay, pointer) = setup();
        controller.set_interactive_regions(vec![region()]);
        pointer.set(0, 0); // fora da janela
        controller.tick();
        controller.tick();
        assert!(!overlay.calls().contains(&true), "ainda não deve passar");
        controller.tick();
        assert_eq!(overlay.calls(), vec![true], "3ª amostra ativa o repasse");
    }

    #[test]
    fn reentry_restores_interactive_immediately() {
        let (controller, overlay, pointer) = setup();
        controller.set_interactive_regions(vec![region()]);
        pointer.set(0, 0);
        controller.tick();
        controller.tick();
        controller.tick();
        assert_eq!(overlay.calls(), vec![true]);

        // Volta para dentro da região → interativo já na próxima amostra.
        pointer.set(120, 120);
        controller.tick();
        assert_eq!(overlay.calls(), vec![true, false]);
    }
}
