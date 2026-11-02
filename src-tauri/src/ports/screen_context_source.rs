//! Porta de leitura do contexto da janela ativa via acessibilidade (BR-105).
//!
//! O domínio (`domain::screen_context`) define o snapshot já podado; esta
//! porta é o contrato de **leitura**. O adaptador do Linux é
//! `AtspiScreenContextSource` (AT-SPI/D-Bus); dobras:
//! `FakeScreenContextSource` e `FailingScreenContextSource`
//! (em `adapters/screen/`).
//!
//! A porta só é instanciada pelo composition root quando
//! `Settings.screen_context_enabled` está ligada (BR-105).

use crate::domain::screen_context::ScreenContextSnapshot;
use crate::error::Result;

/// Lê o contexto da janela ativa/focada pelo sistema de acessibilidade.
pub trait ScreenContextSource: Send + Sync {
    /// Snapshot da janela ativa.
    ///
    /// - `Ok(Some(_))` — há contexto (app, título e texto podados).
    /// - `Ok(None)` — não há contexto agora (janela sem árvore, sem foco
    ///   observado, app negou acesso). **Não** é erro.
    /// - `Err(_)` — o mecanismo falhou (D-Bus de acessibilidade
    ///   inacessível, timeout); a UI vira "Contexto indisponível".
    fn snapshot(&self) -> Result<Option<ScreenContextSnapshot>>;

    /// Nome estável para UI/logs (`"atspi"`); nunca o conteúdo.
    fn name(&self) -> &'static str;
}
