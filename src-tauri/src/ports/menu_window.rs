//! Porta da janela flutuante de menus (agentes/canais).
//!
//! O menu vive em **janela própria**: o notch não muda de tamanho nem de
//! posição ao abrir/fechar (BR-072/ISS-063).

/// Menu exibido na janela flutuante.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    /// Lista de agentes do seletor acima do notch.
    Agent,
    /// Canais de áudio (Ambos/Mic/Sistema + Nova sessão).
    Channel,
}

/// Retângulo em **CSS px**, relativo à janela do overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Gatilho (para o eixo X) e corpo do notch (para o eixo Y).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuAnchor {
    pub trigger: MenuRect,
    pub notch: MenuRect,
}

/// Controla a janela flutuante dos menus.
pub trait MenuWindowController: Send + Sync {
    /// Mostra o menu ancorado no gatilho (reposiciona se já estiver aberto).
    fn open(&self, kind: MenuKind, anchor: MenuAnchor);
    /// Reposiciona/redimensiona com a altura medida pelo conteúdo (px lógicos).
    fn resize(&self, height: f64);
    /// Esconde o menu.
    fn close(&self);
    /// Menu atualmente aberto (para o estado inicial da janela).
    fn current(&self) -> Option<MenuKind>;
}
