//! Reação ao encerramento de uma sessão (inbound port).

/// Observa o fim de uma sessão (**fire-and-forget**).
///
/// Implementado por [`SummaryRefresh`](crate::application::summary_refresh)
/// para garantir que a sessão sai do histórico com resumo/título (BR-104).
/// Quem chama **não** espera: a implementação só agenda o trabalho.
pub trait SessionEndObserver: Send + Sync {
    /// A sessão `session_id` acabou de ser encerrada.
    fn on_session_ended(&self, session_id: &str);
}
