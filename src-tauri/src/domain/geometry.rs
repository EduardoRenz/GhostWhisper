//! Geometria pura usada pelo overlay (sem I/O).
//!
//! Coordenadas de ponteiro e janela são **físicas** (pixels do SO); as regiões
//! interativas chegam da UI em **CSS px** e são escaladas aqui.

/// Ponto no espaço de tela (pixels físicos).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// Retângulo alinhado aos eixos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// `true` quando o ponto está dentro do retângulo (borda superior/esquerda).
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }

    /// Escala o retângulo (CSS px → px físicos).
    pub fn scaled(&self, factor: f64) -> Rect {
        Rect {
            x: (self.x as f64 * factor).round() as i32,
            y: (self.y as f64 * factor).round() as i32,
            width: (self.width as f64 * factor).round() as i32,
            height: (self.height as f64 * factor).round() as i32,
        }
    }

    /// Expande o retângulo por uma margem de conforto (px).
    pub fn expanded(&self, margin: i32) -> Rect {
        Rect {
            x: self.x - margin,
            y: self.y - margin,
            width: self.width + margin * 2,
            height: self.height + margin * 2,
        }
    }
}

/// `true` se o ponto (físico) está em alguma região interativa (CSS px),
/// considerando uma margem de conforto em torno de cada região.
pub fn is_point_interactive_margin(
    pointer: Point,
    window_position: Point,
    scale_factor: f64,
    regions_css: &[Rect],
    margin_css: i32,
) -> bool {
    let local = Point {
        x: pointer.x - window_position.x,
        y: pointer.y - window_position.y,
    };
    regions_css
        .iter()
        .map(|rect| rect.expanded(margin_css).scaled(scale_factor))
        .any(|rect| rect.contains(local))
}

/// Decide se o overlay deve ignorar o cursor (click-through) na posição atual.
///
/// Click-through é ativado quando o ponteiro **não** está sobre nenhuma região
/// interativa — inclusive quando está fora dos limites da janela.
pub fn should_pass_through_margin(
    pointer: Point,
    window_position: Point,
    window_size: (i32, i32),
    scale_factor: f64,
    regions_css: &[Rect],
    margin_css: i32,
) -> bool {
    // Sem regiões informadas pela UI, a janela é totalmente interativa
    // (evita tornar o overlay "morto" antes da UI carregar as regiões).
    if regions_css.is_empty() {
        return false;
    }
    let local = Point {
        x: pointer.x - window_position.x,
        y: pointer.y - window_position.y,
    };
    let inside_window =
        local.x >= 0 && local.y >= 0 && local.x < window_size.0 && local.y < window_size.1;
    if !inside_window {
        return true;
    }
    !is_point_interactive_margin(
        pointer,
        window_position,
        scale_factor,
        regions_css,
        margin_css,
    )
}

/// Resultado do posicionamento da janela de confirmação do drill-hover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DwellPopoverLayout {
    /// Coordenada X (física) do canto superior esquerdo.
    pub x: i32,
    /// Coordenada Y (física) do canto superior esquerdo.
    pub y: i32,
    /// `true` quando abriu acima do gatilho.
    pub above: bool,
}

/// Posiciona a confirmação **acima** do gatilho quando há espaço; senão abaixo.
///
/// `anchor` e `work_area` estão em **pixels físicos**; `size` é o tamanho físico
/// da janela de confirmação. Respeita `gap` (distância do gatilho) e `margin`
/// (borda da área útil), com clamp horizontal/vertical.
pub fn dwell_popover_position(
    anchor: Rect,
    work_area: Rect,
    size: (i32, i32),
    gap: i32,
    margin: i32,
) -> DwellPopoverLayout {
    let min_x = work_area.x + margin;
    let max_x = (work_area.x + work_area.width - size.0 - margin).max(min_x);
    let center_x = anchor.x + anchor.width / 2 - size.0 / 2;
    let x = center_x.clamp(min_x, max_x);

    let space_above = anchor.y - work_area.y;
    let space_below = work_area.y + work_area.height - (anchor.y + anchor.height);
    let needed = size.1 + gap + margin;
    let above = space_above >= needed || (space_below < needed && space_above >= space_below);

    let raw_y = if above {
        anchor.y - gap - size.1
    } else {
        anchor.y + anchor.height + gap
    };
    let min_y = work_area.y + margin;
    let max_y = (work_area.y + work_area.height - size.1 - margin).max(min_y);
    let y = raw_y.clamp(min_y, max_y);

    DwellPopoverLayout { x, y, above }
}

/// Trava de histerese para o click-through.
///
/// Entrar numa região interativa é **imediato** (evita perder o clique — ISS-001);
/// voltar a ignorar o cursor só ocorre após `leave_samples` amostras
/// consecutivas fora das regiões (evita oscilação perto das bordas).
#[derive(Debug, Clone)]
pub struct ClickThroughLatch {
    leave_samples: u32,
    outside_samples: u32,
    passing: bool,
}

impl ClickThroughLatch {
    /// Cria a trava exigindo `leave_samples` amostras (mínimo 1) para reativar.
    pub fn new(leave_samples: u32) -> Self {
        Self {
            leave_samples: leave_samples.max(1),
            outside_samples: 0,
            passing: false,
        }
    }

    /// Atualiza com a intenção da amostra e devolve o estado efetivo.
    ///
    /// `desired_pass_through = true` significa "cursor fora das regiões".
    pub fn update(&mut self, desired_pass_through: bool) -> bool {
        if desired_pass_through {
            self.outside_samples = self.outside_samples.saturating_add(1);
            if self.outside_samples >= self.leave_samples {
                self.passing = true;
            }
        } else {
            self.outside_samples = 0;
            self.passing = false;
        }
        self.passing
    }

    /// Reinicia para o estado interativo.
    pub fn reset(&mut self) {
        self.outside_samples = 0;
        self.passing = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn pass_through(pointer: Point, window: Point, size: (i32, i32), regions: &[Rect]) -> bool {
        should_pass_through_margin(pointer, window, size, 1.0, regions, 0)
    }

    #[test]
    fn contains_respects_half_open_bounds() {
        let r = rect(10, 10, 10, 10);
        assert!(r.contains(Point { x: 10, y: 10 }));
        assert!(r.contains(Point { x: 19, y: 19 }));
        assert!(!r.contains(Point { x: 20, y: 15 }));
        assert!(!r.contains(Point { x: 9, y: 15 }));
    }

    #[test]
    fn pass_through_when_outside_window() {
        let regions = [rect(0, 0, 100, 40)];
        assert!(pass_through(
            Point { x: 500, y: 500 },
            Point { x: 100, y: 100 },
            (200, 80),
            &regions,
        ));
    }

    #[test]
    fn interactive_region_blocks_pass_through() {
        let regions = [rect(10, 10, 50, 20)];
        // ponteiro físico (120,120); janela em (100,100) → local (20,20) dentro.
        assert!(!pass_through(
            Point { x: 120, y: 120 },
            Point { x: 100, y: 100 },
            (200, 80),
            &regions,
        ));
    }

    #[test]
    fn no_regions_keeps_window_interactive() {
        assert!(!pass_through(
            Point { x: 150, y: 150 },
            Point { x: 100, y: 100 },
            (200, 80),
            &[],
        ));
    }

    #[test]
    fn empty_area_inside_window_passes_through() {
        let regions = [rect(10, 10, 50, 20)];
        // local (100,5): dentro da janela, fora da região.
        assert!(pass_through(
            Point { x: 200, y: 105 },
            Point { x: 100, y: 100 },
            (200, 80),
            &regions,
        ));
    }

    #[test]
    fn scales_css_regions_by_scale_factor() {
        let regions = [rect(10, 10, 10, 10)];
        // scale 2 → região física (20,20,20,20); (125,125) com janela (100,100) → local (25,25) dentro.
        assert!(!should_pass_through_margin(
            Point { x: 125, y: 125 },
            Point { x: 100, y: 100 },
            (400, 200),
            2.0,
            &regions,
            0,
        ));
    }

    #[test]
    fn margin_expands_interactive_area() {
        let regions = [rect(20, 20, 10, 10)];
        let pointer = Point { x: 112, y: 112 }; // local (12,12): 8px à esquerda/acima
        let window = Point { x: 100, y: 100 };
        // Sem margem, está fora.
        assert!(pass_through(pointer, window, (200, 80), &regions));
        // Com margem de 8px, entra na área de conforto.
        assert!(!should_pass_through_margin(
            pointer,
            window,
            (200, 80),
            1.0,
            &regions,
            8
        ));
    }

    #[test]
    fn latch_enters_interactive_immediately() {
        let mut latch = ClickThroughLatch::new(3);
        // Cursor dentro de região → interativo já na primeira amostra.
        assert!(!latch.update(false));
    }

    #[test]
    fn latch_debounces_pass_through() {
        let mut latch = ClickThroughLatch::new(3);
        // Fora das regiões: só ativa após 3 amostras.
        assert!(!latch.update(true));
        assert!(!latch.update(true));
        assert!(latch.update(true));
    }

    #[test]
    fn latch_resets_on_reentry() {
        let mut latch = ClickThroughLatch::new(3);
        latch.update(true);
        latch.update(true);
        // Uma amostra dentro da região zera o contador.
        assert!(!latch.update(false));
        assert!(!latch.update(true));
        assert!(!latch.update(true));
        assert!(latch.update(true));
    }

    #[test]
    fn latch_reset_returns_to_interactive() {
        let mut latch = ClickThroughLatch::new(1);
        assert!(latch.update(true));
        latch.reset();
        assert!(!latch.update(false));
    }

    #[test]
    fn dwell_popover_prefers_above_when_it_fits() {
        let anchor = rect(400, 300, 100, 30);
        let work = rect(0, 0, 1920, 1080);
        let layout = dwell_popover_position(anchor, work, (236, 64), 8, 8);
        assert!(layout.above);
        assert_eq!(layout.y, 300 - 8 - 64);
        assert_eq!(layout.x, 400 + 50 - 118);
    }

    #[test]
    fn dwell_popover_falls_below_near_the_top() {
        let anchor = rect(400, 10, 100, 30);
        let work = rect(0, 0, 1920, 1080);
        let layout = dwell_popover_position(anchor, work, (236, 64), 8, 8);
        assert!(!layout.above);
        assert_eq!(layout.y, 10 + 30 + 8);
    }

    #[test]
    fn dwell_popover_clamps_horizontally_to_the_work_area() {
        // Gatilho colado à direita (no meio vertical: acima ainda cabe).
        let anchor = rect(1880, 500, 40, 30);
        let work = rect(0, 0, 1920, 1080);
        let layout = dwell_popover_position(anchor, work, (236, 64), 8, 8);
        assert!(layout.above);
        assert_eq!(layout.x, 1920 - 236 - 8);
    }

    #[test]
    fn dwell_popover_clamps_vertically_in_a_tiny_work_area() {
        let anchor = rect(20, 40, 40, 15);
        let work = rect(0, 0, 300, 100);
        let layout = dwell_popover_position(anchor, work, (236, 64), 8, 8);
        assert!(!layout.above);
        assert_eq!(layout.y, 100 - 64 - 8);
    }

    #[test]
    fn dwell_popover_keeps_position_with_negative_monitor_origin() {
        // Monitor à esquerda do principal (origem negativa).
        let anchor = rect(-300, 100, 80, 30);
        let work = rect(-1920, 0, 1920, 1080);
        let layout = dwell_popover_position(anchor, work, (236, 64), 8, 8);
        assert!(layout.x >= -1920 + 8);
        assert!(layout.above);
    }
}
