//! Adaptador da janela flutuante de menus sobre `tauri::WebviewWindow`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{Emitter, LogicalSize, PhysicalPosition, WebviewWindow};

use crate::domain::geometry::{menu_placement, Rect};
use crate::ports::menu_window::{MenuAnchor, MenuKind, MenuWindowController};

/// Largura lógica do menu de agentes.
const AGENT_MENU_WIDTH: f64 = 300.0;
/// Largura lógica do menu de canais.
const CHANNEL_MENU_WIDTH: f64 = 200.0;
/// Altura inicial (antes da primeira medição) e altura máxima lógica.
const DEFAULT_MENU_HEIGHT: f64 = 200.0;
const MAX_MENU_HEIGHT: f64 = 360.0;
const MIN_MENU_HEIGHT: f64 = 64.0;
/// Distância (px lógicos) entre o corpo do notch e o popup.
const MENU_GAP: i32 = 0;

/// Âncoras físicas guardadas na abertura (para reposicionar ao medir).
#[derive(Clone, Copy)]
struct PhysicalAnchor {
    trigger: Rect,
    notch: Rect,
}

/// Implementa `MenuWindowController` sobre uma janela Tauri própria.
pub struct TauriMenuWindow {
    window: WebviewWindow,
    overlay: WebviewWindow,
    kind: Mutex<Option<MenuKind>>,
    anchor: Mutex<Option<PhysicalAnchor>>,
    /// Última altura (lógica) medida por tipo: agentes, canais.
    heights: Mutex<(f64, f64)>,
    visible: AtomicBool,
}

impl TauriMenuWindow {
    /// Cria o adaptador a partir da janela `menu` e da janela do overlay.
    pub fn new(window: WebviewWindow, overlay: WebviewWindow) -> Self {
        Self {
            window,
            overlay,
            kind: Mutex::new(None),
            anchor: Mutex::new(None),
            heights: Mutex::new((DEFAULT_MENU_HEIGHT, DEFAULT_MENU_HEIGHT)),
            visible: AtomicBool::new(false),
        }
    }

    fn width(kind: MenuKind) -> f64 {
        match kind {
            MenuKind::Agent => AGENT_MENU_WIDTH,
            MenuKind::Channel => CHANNEL_MENU_WIDTH,
        }
    }

    fn height(&self, kind: MenuKind) -> f64 {
        let (agent, channel) = *self.heights.lock().unwrap();
        match kind {
            MenuKind::Agent => agent,
            MenuKind::Channel => channel,
        }
    }

    fn store_height(&self, kind: MenuKind, height: f64) {
        let height = height.clamp(MIN_MENU_HEIGHT, MAX_MENU_HEIGHT);
        let mut heights = self.heights.lock().unwrap();
        match kind {
            MenuKind::Agent => heights.0 = height,
            MenuKind::Channel => heights.1 = height,
        }
    }

    fn kind_id(kind: MenuKind) -> &'static str {
        match kind {
            MenuKind::Agent => "agent",
            MenuKind::Channel => "channel",
        }
    }

    /// Mede as âncoras (CSS → físico) a partir do **cliente** do overlay.
    ///
    /// O GTK/Mutter pode adicionar frame/CSD (ex.: 37 px no topo); o CSS é
    /// relativo ao cliente, então `outer_position` desalinharia o menu.
    fn physical_anchor(&self, anchor: MenuAnchor) -> Option<(PhysicalAnchor, f64)> {
        let position = self
            .overlay
            .inner_position()
            .ok()
            .or_else(|| self.overlay.outer_position().ok())?;
        let scale = self.overlay.scale_factor().unwrap_or(1.0);
        let rect = |value: crate::ports::menu_window::MenuRect| Rect {
            x: position.x + (value.x as f64 * scale).round() as i32,
            y: position.y + (value.y as f64 * scale).round() as i32,
            width: (value.width as f64 * scale).round() as i32,
            height: (value.height as f64 * scale).round() as i32,
        };
        Some((
            PhysicalAnchor {
                trigger: rect(anchor.trigger),
                notch: rect(anchor.notch),
            },
            scale,
        ))
    }

    /// Posiciona a janela com o tamanho atual, mantendo o gap do notch.
    fn place(&self, anchor: PhysicalAnchor, scale: f64, width: f64, height: f64) {
        let menu_size = (
            (width * scale).round() as i32,
            (height * scale).round() as i32,
        );
        let Some(monitor) = self
            .overlay
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| self.overlay.primary_monitor().ok().flatten())
        else {
            return;
        };
        let work = monitor.work_area();
        let work_area = Rect {
            x: work.position.x,
            y: work.position.y,
            width: work.size.width as i32,
            height: work.size.height as i32,
        };
        let gap = (MENU_GAP as f64 * scale).round() as i32;
        let (x, y) = menu_placement(
            anchor.trigger,
            anchor.notch,
            menu_size,
            work_area,
            gap,
            matches!(*self.kind.lock().unwrap(), Some(MenuKind::Channel)),
        );
        // `set_position` posiciona o **cliente** no Linux: `x, y` já são a
        // posição desejada do conteúdo.
        let _ = self.window.set_position(PhysicalPosition::new(x, y));
    }

    /// Reafirma o popup acima do notch (toca o hint `_NET_WM_STATE_ABOVE`).
    fn raise_above(&self) {
        let _ = self.window.set_always_on_top(false);
        let _ = self.window.set_always_on_top(true);
    }

    /// Reforço tardio: o overlay também reafirma o topo e pode subir depois.
    fn schedule_raise(&self) {
        let window = self.window.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            if window.is_visible().unwrap_or(false) {
                let _ = window.set_always_on_top(false);
                let _ = window.set_always_on_top(true);
            }
        });
    }
}

impl MenuWindowController for TauriMenuWindow {
    fn open(&self, kind: MenuKind, anchor: MenuAnchor) {
        let Some((physical, scale)) = self.physical_anchor(anchor) else {
            return;
        };
        let width = Self::width(kind);
        let height = self.height(kind);

        *self.kind.lock().unwrap() = Some(kind);
        *self.anchor.lock().unwrap() = Some(physical);
        self.visible.store(true, Ordering::Relaxed);

        let _ = self.window.set_size(LogicalSize::new(width, height));
        self.place(physical, scale, width, height);

        let _ = self.window.show();
        // O popup nasce depois do notch; garante o topo (ISS-043/BR-073).
        self.raise_above();
        let _ = self.window.set_focus();
        self.schedule_raise();
        // Avisa o webview qual menu renderizar (a janela já está carregada).
        let _ = self.window.emit("menu_show", Self::kind_id(kind));
    }

    fn resize(&self, height: f64) {
        if !height.is_finite() || height <= 0.0 {
            return;
        }
        let Some(kind) = *self.kind.lock().unwrap() else {
            return;
        };
        let Some(anchor) = *self.anchor.lock().unwrap() else {
            return;
        };
        let height = height.clamp(MIN_MENU_HEIGHT, MAX_MENU_HEIGHT);
        if (self.height(kind) - height).abs() < 1.0 {
            return;
        }
        self.store_height(kind, height);
        let width = Self::width(kind);
        let scale = self.overlay.scale_factor().unwrap_or(1.0);
        let _ = self.window.set_size(LogicalSize::new(width, height));
        self.place(anchor, scale, width, height);
    }

    fn close(&self) {
        if !self.visible.swap(false, Ordering::Relaxed) {
            return;
        }
        let _ = self.window.hide();
        *self.kind.lock().unwrap() = None;
        *self.anchor.lock().unwrap() = None;
        let _ = self.window.emit("menu_closed", ());
    }

    fn current(&self) -> Option<MenuKind> {
        *self.kind.lock().unwrap()
    }
}
