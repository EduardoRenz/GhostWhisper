//! Saúde das fontes de IA e **circuit breaker** (BR-045).
//!
//! Cada fonte tem um circuito com três estados:
//!
//! - **fechado** — a fonte é usada; 3 falhas consecutivas abrem o circuito;
//! - **aberto** — a fonte é pulada (sem gastar timeout de rede) por 60 s;
//! - **meio-aberto** — depois do cooldown, a próxima chamada passa e é a
//!   tentativa de recuperação: sucesso fecha o circuito, falha reabre.
//!
//! Erro de **autenticação** (401/403) abre o circuito na hora: repetir uma
//! chave inválida só gasta tempo.
//!
//! O registro é **em memória** e injetado por construtor (sem global
//! mutável — R10); o tempo vem da porta [`crate::ports::clock::Clock`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::domain::ai_source::{AiSourceHealth, AiSourceState};
use crate::error::GhostWhisperError;
use crate::ports::clock::Clock;

/// Falhas consecutivas que abrem o circuito.
pub const FAILURE_THRESHOLD: u32 = 3;

/// Tempo (ms) que o circuito fica aberto antes da tentativa de recuperação.
pub const COOLDOWN_MS: i64 = 60_000;

/// Registro de saúde + circuit breaker das fontes.
pub struct AiSourceHealthRegistry {
    entries: Mutex<HashMap<String, AiSourceHealth>>,
    clock: Arc<dyn Clock>,
}

impl AiSourceHealthRegistry {
    /// Cria o registro com a porta de tempo.
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            clock,
        }
    }

    /// Registra uma chamada bem-sucedida: fecha o circuito e zera as falhas.
    pub fn record_success(&self, source_id: &str, latency_ms: u64) {
        let now = self.clock.now_ms();
        let mut guard = self.entries.lock().expect("registro de saúde");
        let entry = guard
            .entry(source_id.to_string())
            .or_insert_with(|| AiSourceHealth::new(source_id.to_string()));
        entry.state = AiSourceState::Ready;
        entry.consecutive_failures = 0;
        entry.latency_ms = Some(latency_ms);
        entry.last_error = None;
        entry.open_until_ms = None;
        let _ = now;
    }

    /// Registra uma falha: conta, abre o circuito e guarda o motivo.
    pub fn record_failure(&self, source_id: &str, error: &GhostWhisperError) {
        let now = self.clock.now_ms();
        let mut guard = self.entries.lock().expect("registro de saúde");
        let entry = guard
            .entry(source_id.to_string())
            .or_insert_with(|| AiSourceHealth::new(source_id.to_string()));

        // O circuito estava meio-aberto (o cooldown já expirou): reabre.
        let was_half_open = entry.open_until_ms.is_some_and(|until| now >= until);

        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        entry.last_error = Some(error_message(error));
        entry.latency_ms = None;

        let auth_error = is_auth_error(error);
        if was_half_open || auth_error || entry.consecutive_failures >= FAILURE_THRESHOLD {
            entry.state = AiSourceState::Failed;
            entry.open_until_ms = Some(now + COOLDOWN_MS);
        } else {
            entry.state = AiSourceState::Degraded;
        }
    }

    /// `true` quando o circuito está **aberto** (a fonte deve ser pulada).
    ///
    /// Passado o cooldown, o circuito entra em meio-aberto: a chamada passa e
    /// decide se fecha ou reabre.
    pub fn is_open(&self, source_id: &str) -> bool {
        let now = self.clock.now_ms();
        let guard = self.entries.lock().expect("registro de saúde");
        guard
            .get(source_id)
            .and_then(|entry| entry.open_until_ms)
            .is_some_and(|until| now < until)
    }

    /// Marca a fonte como usada em **fallback** (ou como primária).
    pub fn mark_fallback(&self, source_id: &str, is_fallback: bool) {
        let mut guard = self.entries.lock().expect("registro de saúde");
        let entry = guard
            .entry(source_id.to_string())
            .or_insert_with(|| AiSourceHealth::new(source_id.to_string()));
        entry.is_fallback = is_fallback;
    }

    /// Informe de uma fonte (cria um vazio se ela nunca foi usada).
    pub fn health_of(&self, source_id: &str) -> AiSourceHealth {
        let guard = self.entries.lock().expect("registro de saúde");
        guard
            .get(source_id)
            .cloned()
            .unwrap_or_else(|| AiSourceHealth::new(source_id.to_string()))
    }

    /// Informe de todas as fontes conhecidas.
    pub fn snapshot(&self) -> Vec<AiSourceHealth> {
        let guard = self.entries.lock().expect("registro de saúde");
        let mut items: Vec<AiSourceHealth> = guard.values().cloned().collect();
        items.sort_by(|a, b| a.source_id.cmp(&b.source_id));
        items
    }
}

/// `true` quando o erro é de **autenticação** (401/403).
pub fn is_auth_error(error: &GhostWhisperError) -> bool {
    matches!(
        error,
        GhostWhisperError::ProviderStatus {
            code: 401 | 403,
            ..
        }
    )
}

/// Mensagem de erro sem segredos (o `ProviderStatus` já trunca o detalhe).
fn error_message(error: &GhostWhisperError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::system::system_clock::MutableClock;

    fn registry() -> (AiSourceHealthRegistry, Arc<MutableClock>) {
        let clock = Arc::new(MutableClock::new(1_000));
        (AiSourceHealthRegistry::new(clock.clone()), clock)
    }

    fn status(code: u16) -> GhostWhisperError {
        GhostWhisperError::ProviderStatus {
            code,
            detail: "erro".into(),
        }
    }

    #[test]
    fn success_closes_the_circuit_and_records_latency() {
        let (registry, _clock) = registry();
        registry.record_failure("a", &status(500));
        assert_eq!(registry.health_of("a").state, AiSourceState::Degraded);

        registry.record_success("a", 142);
        let health = registry.health_of("a");
        assert_eq!(health.state, AiSourceState::Ready);
        assert_eq!(health.latency_ms, Some(142));
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.last_error.is_none());
    }

    #[test]
    fn three_failures_open_the_circuit() {
        let (registry, _clock) = registry();
        for _ in 0..2 {
            registry.record_failure("a", &status(500));
            assert!(!registry.is_open("a"));
        }
        registry.record_failure("a", &status(500));
        assert!(registry.is_open("a"));
        assert_eq!(registry.health_of("a").state, AiSourceState::Failed);
    }

    #[test]
    fn auth_errors_open_the_circuit_immediately() {
        let (registry, _clock) = registry();
        registry.record_failure("a", &status(401));
        assert!(registry.is_open("a"));
        registry.record_failure("a", &status(403));
        assert!(registry.is_open("a"));
    }

    #[test]
    fn the_circuit_half_opens_after_the_cooldown() {
        let (registry, clock) = registry();
        for _ in 0..3 {
            registry.record_failure("a", &status(500));
        }
        assert!(registry.is_open("a"));

        // Passou o cooldown: a fonte volta a ser tentada.
        clock.advance(COOLDOWN_MS + 1);
        assert!(!registry.is_open("a"));

        // Sucesso fecha o circuito.
        registry.record_success("a", 10);
        assert!(!registry.is_open("a"));
        assert_eq!(registry.health_of("a").state, AiSourceState::Ready);
    }

    #[test]
    fn a_failure_in_half_open_reopens_the_circuit() {
        let (registry, clock) = registry();
        for _ in 0..3 {
            registry.record_failure("a", &status(500));
        }
        clock.advance(COOLDOWN_MS + 1);
        assert!(!registry.is_open("a"));

        registry.record_failure("a", &status(500));
        assert!(registry.is_open("a"));
    }

    #[test]
    fn snapshot_lists_every_known_source() {
        let (registry, _clock) = registry();
        registry.record_success("b", 1);
        registry.record_success("a", 2);
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].source_id, "a");
        assert_eq!(snapshot[1].source_id, "b");
    }

    #[test]
    fn fallback_flag_is_kept() {
        let (registry, _clock) = registry();
        registry.mark_fallback("a", true);
        assert!(registry.health_of("a").is_fallback);
        registry.mark_fallback("a", false);
        assert!(!registry.health_of("a").is_fallback);
    }
}
