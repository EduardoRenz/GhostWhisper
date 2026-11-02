//! Porta do controlador de contexto de tela (BR-105/BR-106).
//!
//! Implementação: `ScreenContextService` (application); dobra:
//! `FakeScreenContextController` (testes).

use crate::domain::screen_context::ScreenContextSnapshot;

/// Disponibilidade do recurso para a linha de status das Configurações.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenContextStatus {
    /// Flag desligada — o adaptador não é instanciado.
    Disabled,
    /// Flag ligada e o mecanismo (AT-SPI/D-Bus) responde.
    Available,
    /// Flag ligada, mas o mecanismo falhou (D-Bus inacessível).
    Unavailable,
}

impl ScreenContextStatus {
    /// Valor serializado para a UI (`snake_case` do contrato IPC).
    pub fn as_str(&self) -> &'static str {
        match self {
            ScreenContextStatus::Disabled => "disabled",
            ScreenContextStatus::Available => "available",
            ScreenContextStatus::Unavailable => "unavailable",
        }
    }
}

/// Controlador do contexto de tela: fonte do snapshot da pill e do prompt.
pub trait ScreenContextController: Send + Sync {
    /// Snapshot para a **pill** (flag mestre ligada **e** checkbox marcado).
    ///
    /// Desmarcado (BR-106), não há leitura nenhuma: devolve `None` sem tocar
    /// na porta — o usuário desligou o contexto para não gastar processamento.
    fn snapshot_for_display(&self) -> Option<ScreenContextSnapshot>;

    /// Snapshot para a **pergunta** (flag ligada + contexto marcado + fresco).
    ///
    /// Devolve `None` quando a flag está desligada, o usuário desmarcou o
    /// checkbox, o mecanismo falhou ou a janela ativa não tem contexto —
    /// nunca erro: o ask segue sem o bloco.
    fn snapshot_for_ask(&self) -> Option<ScreenContextSnapshot>;

    /// Status para a UI de Configurações (linha do card).
    fn status(&self) -> ScreenContextStatus;

    /// Descarta a leitura cacheada e publica "sem contexto" (BR-106).
    ///
    /// Chamado ao **desmarcar** o checkbox: o contexto anterior não pode
    /// reaparecer na próxima abertura nem no próximo ask. Não lê a porta.
    fn clear(&self);
}
