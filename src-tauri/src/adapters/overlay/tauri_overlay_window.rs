//! Adaptador da janela overlay sobre `tauri::WebviewWindow`.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;

use tauri::{LogicalSize, PhysicalPosition, WebviewWindow};

use crate::domain::geometry::{Point, Rect};
use crate::domain::hud_layout::{
    anchor_position, HudDensity, NotchAnchor, AGENT_SELECTOR_BAND, HUD_MARGIN,
};
use crate::ports::overlay_window::{OverlayWindowController, Surface};

/// Largura lógica inicial da barra (antes da medição do conteúdo).
///
/// A UI mede a barra e envia a largura real; este valor só evita uma janela
/// enorme no primeiro frame do boot.
pub const DEFAULT_ACTIONBAR_WIDTH: f64 = 700.0;

/// Limites de segurança da largura dinâmica (px lógicos).
const MIN_ACTIONBAR_WIDTH: f64 = 280.0;
const MAX_ACTIONBAR_WIDTH: f64 = 1_800.0;

/// Implementa `OverlayWindowController` sobre a janela Tauri.
pub struct TauriOverlayWindow {
    window: WebviewWindow,
    click_through: Arc<AtomicBool>,
    anchor_bottom: AtomicBool,
    /// Largura lógica atual da barra (medida pela UI).
    width: AtomicU64,
    /// Densidade do HUD (BR-062): 0 compacto · 1 padrão · 2 expandido.
    density: AtomicU8,
    /// Ancoragem do notch (BR-062): 0 topo central · 1 canto direito · 2 livre.
    anchor: AtomicU8,
    /// Estado desejado do stealth, para ser **reafirmado** quando o compositor
    /// ou o Tauri remapeiam a janela (Perssua: `scheduleOverlayTaskbarStateReassertion`).
    stealth: AtomicBool,
}

impl TauriOverlayWindow {
    /// Cria o adaptador a partir de uma janela Tauri existente.
    pub fn new(window: WebviewWindow) -> Self {
        Self {
            window,
            click_through: Arc::new(AtomicBool::new(false)),
            anchor_bottom: AtomicBool::new(true),
            width: AtomicU64::new(DEFAULT_ACTIONBAR_WIDTH.to_bits()),
            density: AtomicU8::new(1),
            anchor: AtomicU8::new(0),
            stealth: AtomicBool::new(false),
        }
    }

    /// Largura lógica atual da barra.
    fn bar_width(&self) -> f64 {
        f64::from_bits(self.width.load(Ordering::Relaxed))
    }

    /// Altura da pilha do notch (seletor + barra + banner), sem a reserva.
    fn base_height(&self) -> f64 {
        AGENT_SELECTOR_BAND + self.density().window_height()
    }

    /// Altura total da janela: seletor de agentes + barra (densidade).
    fn window_height(&self) -> f64 {
        self.base_height()
    }

    /// Aplica largura/altura atuais à janela.
    fn apply_size(&self) {
        let _ = self
            .window
            .set_size(LogicalSize::new(self.bar_width(), self.window_height()));
    }

    /// Densidade atual do HUD.
    fn density(&self) -> HudDensity {
        match self.density.load(Ordering::Relaxed) {
            0 => HudDensity::Compact,
            2 => HudDensity::Expanded,
            _ => HudDensity::Standard,
        }
    }

    /// Ancoragem atual do notch.
    fn anchor(&self) -> NotchAnchor {
        match self.anchor.load(Ordering::Relaxed) {
            1 => NotchAnchor::TopRight,
            2 => NotchAnchor::Free,
            _ => NotchAnchor::TopCenter,
        }
    }

    fn store_density(&self, density: HudDensity) {
        let code = match density {
            HudDensity::Compact => 0,
            HudDensity::Standard => 1,
            HudDensity::Expanded => 2,
        };
        self.density.store(code, Ordering::Relaxed);
    }

    fn store_anchor(&self, anchor: NotchAnchor) {
        let code = match anchor {
            NotchAnchor::TopCenter => 0,
            NotchAnchor::TopRight => 1,
            NotchAnchor::Free => 2,
        };
        self.anchor.store(code, Ordering::Relaxed);
    }

    /// Reposiciona o notch pela área útil do monitor primário (BR-062).
    fn apply_anchor(&self) {
        let anchor = self.anchor();
        if anchor.is_anchored() {
            self.place_at(anchor);
        }
    }

    /// Coloca a janela na posição de uma âncora fixa (sem mudar a preferida).
    fn place_at(&self, anchor: NotchAnchor) {
        let Ok(Some(monitor)) = self.window.primary_monitor() else {
            return;
        };
        let scale = monitor.scale_factor();
        let work = monitor.work_area();
        let work_area = Rect {
            x: work.position.x,
            y: work.position.y,
            width: work.size.width as i32,
            height: work.size.height as i32,
        };
        let width = (self.bar_width() * scale).round() as i32;
        let height = (self.window_height() * scale).round() as i32;
        let margin = (HUD_MARGIN as f64 * scale).round() as i32;
        if let Some((x, y)) = anchor_position(anchor, work_area, (width, height), margin) {
            let _ = self.window.set_position(PhysicalPosition::new(x, y));
        }
    }

    /// Aplica (ou reaplica) o estado de stealth na janela.
    ///
    /// `set_content_protected` é paridade com o Perssua
    /// (`setContentProtection`): no **Linux é no-op** (a API só tem efeito no
    /// macOS/Windows), então não prometemos invisibilidade no compartilhamento
    /// (BR-006) — o efeito real aqui é sair da barra de tarefas e do título.
    fn apply_stealth(&self) {
        let enabled = self.stealth.load(Ordering::Relaxed);
        let _ = self.window.set_skip_taskbar(enabled);
        let _ = self
            .window
            .set_title(if enabled { "" } else { "GhostWhisper" });
        let _ = self.window.set_content_protected(enabled);
        // O `alwaysOnTop` do `tauri.conf.json` vale na criação, mas o GTK/Mutter
        // perde o `_NET_WM_STATE_ABOVE` quando a janela é mapeada/remodelada
        // (ISS-043). Reafirmar aqui cobre o `show()` e todo `set_surface`.
        self.apply_stacking();
    }

    /// Reafirma o empilhamento da janela (sempre à frente — ISS-043).
    fn apply_stacking(&self) {
        let _ = self.window.set_always_on_top(true);
    }
}

impl OverlayWindowController for TauriOverlayWindow {
    fn show(&self) {
        let _ = self.window.show();
        // Reafirma o stealth: alguns compositores (e o próprio remapeamento)
        // readicionam a janela à barra de tarefas / devolvem o título.
        self.apply_stealth();
    }

    fn hide(&self) {
        let _ = self.window.hide();
    }

    fn toggle(&self) -> bool {
        if self.is_visible() {
            self.hide();
            false
        } else {
            self.show();
            true
        }
    }

    fn is_visible(&self) -> bool {
        self.window.is_visible().unwrap_or(false)
    }

    fn set_click_through(&self, enabled: bool) {
        // Evita chamadas redundantes ao compositor.
        if self.click_through.swap(enabled, Ordering::Relaxed) == enabled {
            return;
        }
        let _ = self.window.set_ignore_cursor_events(enabled);
    }

    fn set_stealth(&self, enabled: bool) {
        self.stealth.store(enabled, Ordering::Relaxed);
        self.apply_stealth();
    }

    fn set_surface(&self, surface: Surface) {
        if matches!(surface, Surface::Hidden) {
            self.hide();
            return;
        }

        self.apply_size();

        // Âncora fixa (BR-062) manda na posição. Em "Flutuante Livre" o
        // redimensionamento mantém o canto superior (a barra vive no topo) —
        // não há mais reancoragem pela base.
        if self.anchor().is_anchored() {
            self.apply_anchor();
        }

        // Redimensionar remapeia a janela em alguns compositores: reafirma o
        // stealth (T19).
        self.apply_stealth();
    }

    fn set_hud_layout(&self, density: HudDensity, anchor: NotchAnchor) {
        self.store_density(density);
        self.store_anchor(anchor);
        self.apply_size();
        if anchor.is_anchored() {
            self.apply_anchor();
        } else if self.anchor_bottom.load(Ordering::Relaxed) {
            // "Livre" sem posição do usuário: começa no topo central (BR-062).
            self.place_at(NotchAnchor::TopCenter);
        }
    }

    fn set_actionbar_width(&self, width: f64) {
        if !width.is_finite() || width <= 0.0 {
            return;
        }
        // Nunca deixa a barra maior que a área útil do monitor.
        let max = self
            .window
            .primary_monitor()
            .ok()
            .flatten()
            .map(|monitor| {
                (monitor.work_area().size.width as f64 / monitor.scale_factor())
                    .clamp(MIN_ACTIONBAR_WIDTH, MAX_ACTIONBAR_WIDTH)
            })
            .unwrap_or(MAX_ACTIONBAR_WIDTH);
        let width = width.clamp(MIN_ACTIONBAR_WIDTH, max);
        let previous = self.bar_width();
        if (previous - width).abs() < 0.5 {
            return;
        }
        self.width.store(width.to_bits(), Ordering::Relaxed);
        self.apply_size();
        if self.anchor().is_anchored() {
            self.apply_anchor();
        }
        // Resize pode remapear e perder o always-on-top (ISS-043).
        self.apply_stealth();
    }

    fn set_anchor_bottom(&self, anchored: bool) {
        self.anchor_bottom.store(anchored, Ordering::Relaxed);
    }

    fn anchor_bottom(&self) -> bool {
        self.anchor_bottom.load(Ordering::Relaxed)
    }

    fn outer_position(&self) -> Option<Point> {
        self.window
            .outer_position()
            .ok()
            .map(|p| Point { x: p.x, y: p.y })
    }

    fn outer_size(&self) -> Option<(i32, i32)> {
        self.window
            .outer_size()
            .ok()
            .map(|s| (s.width as i32, s.height as i32))
    }

    fn scale_factor(&self) -> f64 {
        self.window.scale_factor().unwrap_or(1.0)
    }
}
