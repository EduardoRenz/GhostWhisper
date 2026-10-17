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
