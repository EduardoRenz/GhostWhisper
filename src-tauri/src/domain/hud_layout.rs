//! Layout do HUD (BR-062): ancoragem e densidade do notch, sem I/O.
//!
//! Coordenadas e tamanhos são **físicos** (pixels do SO). A área útil do monitor
//! (work area) entra pronta; aqui só há aritmética — o adaptador da janela é
//! quem pergunta ao SO.

use crate::domain::geometry::Rect;

/// Ponto de fixação do notch na área útil do monitor primário.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotchAnchor {
    /// Centralizado no topo (padrão).
    TopCenter,
    /// Encostado no canto superior direito.
    TopRight,
    /// Posição livre: vale a posição lembrada e o arraste.
    Free,
}

/// Espaçamento interno/espessura da barra do notch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudDensity {
    /// Barra enxuta (62 px de janela).
    Compact,
    /// Padrão (72 px de janela).
    Standard,
    /// Barra mais alta (84 px de janela).
    Expanded,
}

impl NotchAnchor {
    /// Todos os valores aceitos, na ordem de exibição.
    pub const ALL: [NotchAnchor; 3] = [
        NotchAnchor::TopCenter,
        NotchAnchor::TopRight,
        NotchAnchor::Free,
    ];

    /// Rótulo persistido (`top_center`, `top_right`, `free`).
    pub fn as_str(self) -> &'static str {
        match self {
            NotchAnchor::TopCenter => "top_center",
            NotchAnchor::TopRight => "top_right",
            NotchAnchor::Free => "free",
        }
    }

    /// Converte o rótulo persistido; desconhecido cai no padrão.
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "top_right" => NotchAnchor::TopRight,
            "free" => NotchAnchor::Free,
            _ => NotchAnchor::TopCenter,
        }
    }

    /// `true` quando a posição é definida pela configuração (não pelo usuário).
    pub fn is_anchored(self) -> bool {
        !matches!(self, NotchAnchor::Free)
    }
}

impl HudDensity {
    /// Todas as densidades, na ordem de exibição.
    pub const ALL: [HudDensity; 3] = [
        HudDensity::Compact,
        HudDensity::Standard,
        HudDensity::Expanded,
    ];

    /// Altura da **janela** (px lógicos): barra + respiro do aviso do ASR.
    ///
    /// A barra fica no **topo** da janela; o excedente é o espaço (invisível)
    /// abaixo dela para o banner/erro (BR-062).
    pub fn window_height(self) -> f64 {
        match self {
            HudDensity::Compact => 72.0,
            HudDensity::Standard => 84.0,
            HudDensity::Expanded => 96.0,
        }
    }

    /// Rótulo persistido (`compact`, `standard`, `expanded`).
    pub fn as_str(self) -> &'static str {
        match self {
            HudDensity::Compact => "compact",
            HudDensity::Standard => "standard",
            HudDensity::Expanded => "expanded",
        }
    }

    /// Converte o rótulo persistido; desconhecido cai no padrão.
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "compact" => HudDensity::Compact,
            "expanded" => HudDensity::Expanded,
            _ => HudDensity::Standard,
        }
    }
}

/// Margem superior/da borda (px lógicos) entre a área útil e o notch.
///
/// Zero: a barra encosta no topo da tela (o usuário pode arrastar para baixo).
pub const HUD_MARGIN: i32 = 0;

/// Posição física do notch para uma âncora fixa.
///
/// `work_area` é a área útil do monitor (já em pixels físicos), `window_size` é
/// o tamanho físico da janela e `margin` a distância das bordas. Devolve `None`
/// em [`NotchAnchor::Free`] — nesse modo a posição é do usuário.
pub fn anchor_position(
    anchor: NotchAnchor,
    work_area: Rect,
    window_size: (i32, i32),
    margin: i32,
) -> Option<(i32, i32)> {
    let (window_width, _) = window_size;
    let y = work_area.y + margin;
    match anchor {
        NotchAnchor::TopCenter => Some((work_area.x + (work_area.width - window_width) / 2, y)),
        NotchAnchor::TopRight => Some((work_area.x + work_area.width - window_width - margin, y)),
        NotchAnchor::Free => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn top_center_centraliza() {
        let position = anchor_position(NotchAnchor::TopCenter, work(), (1040, 84), 0);
        assert_eq!(position, Some((440, 0)));
    }

    #[test]
    fn top_right_encosta_a_direita() {
        let position = anchor_position(NotchAnchor::TopRight, work(), (1040, 84), 0);
        assert_eq!(position, Some((880, 0)));
    }

    #[test]
    fn free_nao_define_posicao() {
        assert_eq!(
            anchor_position(NotchAnchor::Free, work(), (1040, 72), 8),
            None
        );
    }

    #[test]
    fn respeita_origem_e_area_util_do_monitor() {
        // Monitor secundário à direita, com barra de tarefas de 40 px no topo.
        let work = Rect {
            x: 1920,
            y: 40,
            width: 2560,
            height: 1400,
        };
        assert_eq!(
            anchor_position(NotchAnchor::TopCenter, work, (1040, 84), 0),
            Some((1920 + 760, 40))
        );
        assert_eq!(
            anchor_position(NotchAnchor::TopRight, work, (1040, 84), 0),
            Some((1920 + 2560 - 1040, 40))
        );
    }

    #[test]
    fn densidades_tem_alturas_estaveis() {
        assert_eq!(HudDensity::Compact.window_height(), 72.0);
        assert_eq!(HudDensity::Standard.window_height(), 84.0);
        assert_eq!(HudDensity::Expanded.window_height(), 96.0);
    }

    #[test]
    fn parse_cai_no_padrao() {
        assert_eq!(NotchAnchor::parse("top_right"), NotchAnchor::TopRight);
        assert_eq!(NotchAnchor::parse("qualquer"), NotchAnchor::TopCenter);
        assert_eq!(HudDensity::parse("expanded"), HudDensity::Expanded);
        assert_eq!(HudDensity::parse(""), HudDensity::Standard);
    }

    #[test]
    fn rotulos_de_ida_e_volta() {
        for anchor in NotchAnchor::ALL {
            assert_eq!(NotchAnchor::parse(anchor.as_str()), anchor);
        }
        for density in HudDensity::ALL {
            assert_eq!(HudDensity::parse(density.as_str()), density);
        }
    }

    #[test]
    fn ancorada_nao_e_livre() {
        assert!(NotchAnchor::TopCenter.is_anchored());
        assert!(NotchAnchor::TopRight.is_anchored());
        assert!(!NotchAnchor::Free.is_anchored());
    }
}
