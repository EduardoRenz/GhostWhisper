//! Dobras da porta de contexto de tela (BR-105) — sem D-Bus, sem AT-SPI.
//!
//! `FakeScreenContextSource` devolve snapshots configurados e registra as
//! chamadas; `FailingScreenContextSource` simula D-Bus inacessível.
//
// TODO(task 06+): remover o `allow(dead_code)` quando mais consumidores
// (ex.: integração) usarem as dobras fora dos testes do serviço.
#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::domain::screen_context::ScreenContextSnapshot;
use crate::error::Result;
use crate::ports::screen_context_source::ScreenContextSource;

/// Dobra determinística: devolve o snapshot enfileirado e conta chamadas.
///
/// Sem snapshot configurado (ou após o último), devolve `Ok(None)` — o
/// comportamento de "janela sem contexto".
pub struct FakeScreenContextSource {
    snapshot: Mutex<Option<ScreenContextSnapshot>>,
    calls: AtomicUsize,
}

impl FakeScreenContextSource {
    /// Cria a dobra sem snapshot (`Ok(None)`).
    pub fn new() -> Self {
        Self {
            snapshot: Mutex::new(None),
            calls: AtomicUsize::new(0),
        }
    }

    /// Define o snapshot devolvido por `snapshot()`.
    pub fn with_snapshot(self, snapshot: ScreenContextSnapshot) -> Self {
        if let Ok(mut slot) = self.snapshot.lock() {
            *slot = Some(snapshot);
        }
        self
    }

    /// Quantas vezes `snapshot()` foi chamado.
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Default for FakeScreenContextSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenContextSource for FakeScreenContextSource {
    fn snapshot(&self) -> Result<Option<ScreenContextSnapshot>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let slot = self.snapshot.lock().expect("lock");
        Ok(slot.clone())
    }

    fn name(&self) -> &'static str {
        "fake"
    }
}

/// Dobra de falha: sempre devolve erro de mecanismo (UI → "indisponível").
pub struct FailingScreenContextSource {
    calls: AtomicUsize,
}

impl FailingScreenContextSource {
    pub fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }

    /// Quantas vezes `snapshot()` foi chamado.
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Default for FailingScreenContextSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenContextSource for FailingScreenContextSource {
    fn snapshot(&self) -> Result<Option<ScreenContextSnapshot>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(crate::error::GhostWhisperError::Other(
            "atspi indisponível (fake)".into(),
        ))
    }

    fn name(&self) -> &'static str {
        "failing"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(text: &str) -> ScreenContextSnapshot {
        ScreenContextSnapshot {
            app: "chrome".into(),
            window_title: "Doc".into(),
            text: text.into(),
            captured_at_ms: 1_000,
        }
    }

    #[test]
    fn fake_returns_configured_snapshot_and_counts_calls() {
        let fake = FakeScreenContextSource::new().with_snapshot(snapshot("hello"));

        assert_eq!(fake.calls(), 0);
        let got = fake.snapshot().expect("ok");
        assert_eq!(got.as_ref().map(|s| s.text.as_str()), Some("hello"));
        assert_eq!(fake.calls(), 1);
        assert_eq!(fake.name(), "fake");
    }

    #[test]
    fn fake_without_snapshot_yields_none() {
        let fake = FakeScreenContextSource::new();
        assert!(fake.snapshot().expect("ok").is_none());
    }

    #[test]
    fn failing_source_surfaces_error() {
        let failing = FailingScreenContextSource::new();
        assert!(failing.snapshot().is_err());
        assert_eq!(failing.calls(), 1);
    }
}
