//! Porta de tempo (permite testes determinísticos).

/// Fornece o tempo atual.
///
/// Adaptador: `SystemClock`; dobra de teste: `FixedClock`.
pub trait Clock: Send + Sync {
    /// Milissegundos desde a época Unix.
    fn now_ms(&self) -> i64;
}
