//! Decisão **pura** de quando gerar uma sugestão automática.
//!
//! Transposto do Perssua: os insights automáticos não dependem de uma janela
//! aberta — eles reagem a **fala nova**, com um pequeno *debounce* depois do fim
//! do turno e um teto de frequência para limitar custo (BR-028).
//!
//! A thread que executa vive no composition root; aqui só a decisão (R7: sem
//! I/O, testável sem threads nem relógio).

/// Observação de um instante do agendador.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerTick {
    /// O usuário ligou a sugestão automática (`Settings.suggestion_auto_refresh`).
    pub enabled: bool,
    /// Há sessão aberta.
    pub has_session: bool,
    /// Até onde já foi sugerido (cursor; `None` = nada ainda).
    pub cursor_ms: Option<i64>,
    /// Início da última fala **finalizada** com texto.
    pub latest_final_ms: Option<i64>,
    /// Tempo desde a última execução do agendador (ms).
    pub ms_since_last_run: i64,
    /// Tempo desde o início da última fala finalizada (ms) — o *debounce*.
    pub ms_since_latest_final: i64,
}

/// Decide se é hora de gerar uma sugestão automática.
///
/// Regras (todas precisam valer):
/// - ligado e com sessão aberta;
/// - existe fala finalizada **depois** do cursor (há conteúdo novo);
/// - passou `debounce_ms` desde o início da última fala (o turno fechou);
/// - passou `min_interval_ms` desde a última execução (limita custo).
pub fn should_trigger(tick: &SchedulerTick, debounce_ms: i64, min_interval_ms: i64) -> bool {
    if !tick.enabled || !tick.has_session {
        return false;
    }
    let Some(latest) = tick.latest_final_ms else {
        return false;
    };
    if let Some(cursor) = tick.cursor_ms {
        if latest <= cursor {
            return false;
        }
    }
    if tick.ms_since_latest_final < debounce_ms {
        return false;
    }
    tick.ms_since_last_run >= min_interval_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick() -> SchedulerTick {
        SchedulerTick {
            enabled: true,
            has_session: true,
            cursor_ms: None,
            latest_final_ms: Some(10_000),
            ms_since_last_run: 60_000,
            ms_since_latest_final: 5_000,
        }
    }

    const DEBOUNCE: i64 = 800;
    const MIN_INTERVAL: i64 = 3_000;

    #[test]
    fn triggers_with_new_speech_after_the_debounce() {
        assert!(should_trigger(&tick(), DEBOUNCE, MIN_INTERVAL));
    }

    #[test]
    fn does_not_trigger_without_session_or_when_disabled() {
        assert!(!should_trigger(
            &SchedulerTick {
                enabled: false,
                ..tick()
            },
            DEBOUNCE,
            MIN_INTERVAL
        ));
        assert!(!should_trigger(
            &SchedulerTick {
                has_session: false,
                ..tick()
            },
            DEBOUNCE,
            MIN_INTERVAL
        ));
    }

    #[test]
    fn does_not_trigger_without_new_speech() {
        let s = SchedulerTick {
            cursor_ms: Some(10_000),
            ..tick()
        };
        assert!(!should_trigger(&s, DEBOUNCE, MIN_INTERVAL));
        let s = SchedulerTick {
            latest_final_ms: None,
            ..tick()
        };
        assert!(!should_trigger(&s, DEBOUNCE, MIN_INTERVAL));
    }

    #[test]
    fn waits_for_the_turn_to_close_before_firing() {
        let s = SchedulerTick {
            ms_since_latest_final: 300,
            ..tick()
        };
        assert!(!should_trigger(&s, DEBOUNCE, MIN_INTERVAL));
    }

    #[test]
    fn respects_the_minimum_interval_between_runs() {
        let s = SchedulerTick {
            ms_since_last_run: 500,
            ..tick()
        };
        assert!(!should_trigger(&s, DEBOUNCE, MIN_INTERVAL));
        let s = SchedulerTick {
            ms_since_last_run: MIN_INTERVAL,
            ..tick()
        };
        assert!(should_trigger(&s, DEBOUNCE, MIN_INTERVAL));
    }
}
