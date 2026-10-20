//! Porta de controle da janela overlay.

use crate::domain::geometry::Point;
use crate::domain::hud_layout::{HudDensity, NotchAnchor};

/// Superfície visível do overlay.
///
/// A conversa e o histórico saíram do overlay para janelas próprias (BR-026);
/// sobraram a barra de ações e o esconder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// Barra fina (ação rápida).
    ActionBar,
    /// Oculto.
    Hidden,
}

/// Controla a janela overlay (mostrar/ocultar, stealth, click-through).
///
/// Implementações concretas (adapters) prefixadas pela tecnologia:
/// `TauriOverlayWindow`.
pub trait OverlayWindowController: Send + Sync {
    /// Mostra a janela sem roubar foco.
    fn show(&self);
    /// Oculta a janela.
    fn hide(&self);
    /// Alterna visibilidade; retorna o novo estado (`true` = visível).
    fn toggle(&self) -> bool;
    /// Estado atual de visibilidade.
    fn is_visible(&self) -> bool;
    /// Ativa/desativa o repasse de cliques (click-through).
    fn set_click_through(&self, enabled: bool);
    /// Liga/desliga o modo stealth (fora da barra de tarefas, sem título).
    fn set_stealth(&self, enabled: bool);
    /// Ajusta a superfície (tamanho/visibilidade) da janela.
    fn set_surface(&self, surface: Surface);
    /// Aplica a densidade e a ancoragem do HUD (BR-062).
    ///
    /// Âncoras fixas reposicionam a janela na área útil do monitor primário;
    /// `Free` preserva a posição atual.
    fn set_hud_layout(&self, density: HudDensity, anchor: NotchAnchor);
    /// Define se o redimensionamento deve reancorar pela base.
    ///
    /// `false` após o usuário mover a janela (posição passa a ser preservada).
    fn set_anchor_bottom(&self, anchored: bool);
    /// Estado atual de ancoragem pela base.
    fn anchor_bottom(&self) -> bool;
    /// Posição física da janela no espaço de tela.
    fn outer_position(&self) -> Option<Point>;
    /// Tamanho físico da janela `(largura, altura)`.
    fn outer_size(&self) -> Option<(i32, i32)>;
    /// Fator de escala do monitor da janela.
    fn scale_factor(&self) -> f64;
}
