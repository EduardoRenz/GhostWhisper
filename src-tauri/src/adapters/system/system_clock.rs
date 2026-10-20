//! Relógio do sistema.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::ports::clock::Clock;

/// Implementa `Clock` com o relógio do sistema.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
pub struct FixedClock(pub i64);

#[cfg(test)]
impl Clock for FixedClock {
    fn now_ms(&self) -> i64 {
        self.0
    }
}

/// Relógio de teste que **avança** sob demanda (circuito do provider).
#[cfg(test)]
pub struct MutableClock(std::sync::atomic::AtomicI64);

#[cfg(test)]
impl MutableClock {
    /// Cria o relógio em `now_ms`.
    pub fn new(now_ms: i64) -> Self {
        Self(std::sync::atomic::AtomicI64::new(now_ms))
    }

    /// Avança o relógio em `ms` milissegundos.
    pub fn advance(&self, ms: i64) {
        self.0.fetch_add(ms, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
impl Clock for MutableClock {
    fn now_ms(&self) -> i64 {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
}
