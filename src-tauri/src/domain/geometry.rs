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

/// Posição de um popup ancorado (menus de agentes/canais), em pixels físicos.
///
/// `trigger` alinha o eixo X (centro ou direita do botão); `notch` manda no
/// eixo Y: o menu fica **abaixo do corpo do notch** ou, se não couber, **acima
/// dele** — nunca por cima da barra. O popup é uma janela própria: o notch não
/// muda de tamanho nem de posição (BR-072/ISS-062).
pub fn menu_placement(
    trigger: Rect,
    notch: Rect,
    menu_size: (i32, i32),
    work_area: Rect,
    gap: i32,
    align_right: bool,
) -> (i32, i32) {
    let (menu_w, menu_h) = menu_size;
    let gap = gap.max(0);
    let min_x = work_area.x;
    let max_x = (work_area.x + work_area.width - menu_w).max(min_x);
    let min_y = work_area.y;
    let max_y = (work_area.y + work_area.height - menu_h).max(min_y);

    let x = if align_right {
        trigger.x + trigger.width - menu_w
    } else {
        trigger.x + (trigger.width - menu_w) / 2
    }
    .clamp(min_x, max_x);

    // Abaixo do **corpo do notch** (barra/pill), não do botão: evita cobrir a
    // barra e ficar atrás dela.
    let below = notch.y + notch.height + gap;
    let above = notch.y - gap - menu_h;
    let y = if below + menu_h <= work_area.y + work_area.height {
        below
    } else if above >= work_area.y {
        above
    } else {
        below
    }
    .clamp(min_y, max_y);
    (x, y)
}

/// Trava de histerese do click-through.
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
    fn menu_opens_below_the_notch_when_it_fits() {
        let work = rect(0, 0, 1920, 1080);
        let trigger = rect(900, 200, 100, 30);
        let notch = rect(800, 180, 300, 100);
        // menu 200x120: abaixo do notch (280+6+120 = 406 ≤ 1080); x centrado no gatilho.
        assert_eq!(
            menu_placement(trigger, notch, (200, 120), work, 6, false),
            (850, 286)
        );
    }

    #[test]
    fn menu_opens_above_the_notch_near_the_bottom() {
        let work = rect(0, 0, 1920, 1080);
        let trigger = rect(900, 1000, 100, 30);
        let notch = rect(800, 980, 300, 100);
        // abaixo iria a 1086 > 1080; abre acima do notch: 980-6-120 = 854.
        assert_eq!(
            menu_placement(trigger, notch, (200, 120), work, 6, false),
            (850, 854)
        );
    }

    #[test]
    fn menu_clamps_x_and_right_aligns() {
        let work = rect(0, 0, 1920, 1080);
        let notch = rect(1800, 100, 120, 40);
        // Colado à direita: alinhado à direita do gatilho e clampado.
        let anchor = rect(1880, 100, 30, 30);
        assert_eq!(
            menu_placement(anchor, notch, (220, 100), work, 6, true).0,
            1690
        );
        // Colado à esquerda, centralizado: clampa em 0.
        let anchor = rect(10, 100, 30, 30);
        assert_eq!(
            menu_placement(anchor, notch, (220, 100), work, 6, false).0,
            0
        );
    }

    #[test]
    fn menu_respects_work_area_origin() {
        let work = rect(1920, 40, 1920, 1040);
        let notch = rect(2900, 100, 300, 100);
        let anchor = rect(3000, 100, 100, 30);
        let (x, y) = menu_placement(anchor, notch, (220, 100), work, 6, false);
        assert!(x >= 1920 && x + 220 <= 3840);
        assert!(y >= 40 && y + 100 <= 1080);
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
}
