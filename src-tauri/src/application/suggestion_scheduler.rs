//! Decisão **pura** de quando gerar uma sugestão automática.
//!
//! O intervalo das settings (`suggestion_interval_secs`) é o **tempo mínimo
//! entre sugestões automáticas** (um *cooldown*), contado desde a última
//! geração ou desde a abertura da janela de Sugestões. A contagem **não**
//! reinicia a cada fala nova: fala nova é só o **conteúdo** que habilita a
//! próxima sugestão. Quando o contador zera:
//!
//! - com fala nova desde a última sugestão, gera;
//! - sem fala nova, fica em `awaiting_speech` e gera assim que a próxima fala
//!   finalizar (sem esperar outro intervalo cheio).
//!
//! **Abrir a janela também reinicia o contador** (pedido do usuário): a janela
//! não gera ao abrir; o ciclo começa cheio e o auto tem tempo de ler a conversa.
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
    /// Fim da última fala **finalizada** com texto.
    pub latest_final_ms: Option<i64>,
    /// Tempo desde o início do ciclo atual (última geração ou abertura da
    /// janela), em ms.
    pub elapsed_ms: i64,
}

/// Decide se é hora de gerar uma sugestão automática.
///
/// Regras (todas precisam valer):
/// - ligado e com sessão aberta;
/// - o intervalo (cooldown) desde a última geração/abertura já passou;
/// - existe fala finalizada **depois** do cursor (há conteúdo novo).
///
/// Fala nova **não** reinicia a contagem; ela só libera a próxima geração.
pub fn should_trigger(tick: &SchedulerTick, interval_ms: i64) -> bool {
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
    tick.elapsed_ms.max(0) >= interval_ms
}

/// Cadência do auto-sugerir: o que a janela de sugestões mostra (BR-039).
///
/// É o **mesmo timer** que [`should_trigger`] usa — a cadência não muda, só
/// fica visível. A função é pura (R7): o agendador apenas publica o resultado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cadence {
    /// O automático está ligado.
    pub enabled: bool,
    /// O automático está rodando (ligado e com sessão aberta).
    pub running: bool,
    /// Milissegundos até a próxima sugestão.
    pub remaining_ms: i64,
    /// O intervalo venceu, mas não há fala nova para sugerir.
    pub awaiting_speech: bool,
    /// Duração total do ciclo (desenha o anel de progresso).
    pub window_ms: i64,
}

/// Calcula a cadência a partir de um tick do agendador.
pub fn cadence(tick: &SchedulerTick, interval_ms: i64) -> Cadence {
    let window_ms = interval_ms.max(1);
    if !tick.enabled || !tick.has_session {
        return Cadence {
            enabled: tick.enabled,
            running: false,
            remaining_ms: 0,
            awaiting_speech: false,
            window_ms,
        };
    }

    let remaining = interval_ms.saturating_sub(tick.elapsed_ms.max(0)).max(0);
    let has_new_speech = match tick.latest_final_ms {
        Some(latest) => tick.cursor_ms.map(|cursor| latest > cursor).unwrap_or(true),
        None => false,
    };

    if remaining > 0 {
        return Cadence {
            enabled: true,
            running: true,
            remaining_ms: remaining,
            awaiting_speech: false,
            window_ms,
        };
    }
    if !has_new_speech {
        // Nada novo desde a última sugestão: espera a próxima fala.
        return Cadence {
            enabled: true,
            running: true,
            remaining_ms: 0,
            awaiting_speech: true,
            window_ms,
        };
    }

    // Intervalo vencido com fala nova: a geração dispara neste tick.
    Cadence {
        enabled: true,
        running: true,
        remaining_ms: 0,
        awaiting_speech: false,
        window_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick() -> SchedulerTick {
        SchedulerTick {
            enabled: true,
            has_session: true,
            cursor_ms: None,
            latest_final_ms: Some(20_000),
            // O ciclo já venceu.
            elapsed_ms: 25_000,
        }
    }

    const INTERVAL: i64 = 20_000;

    #[test]
    fn triggers_with_new_speech_after_the_interval() {
        assert!(should_trigger(&tick(), INTERVAL));
    }

    #[test]
    fn does_not_trigger_without_session_or_when_disabled() {
        assert!(!should_trigger(
            &SchedulerTick {
                enabled: false,
                ..tick()
            },
            INTERVAL
        ));
        assert!(!should_trigger(
            &SchedulerTick {
                has_session: false,
                ..tick()
            },
            INTERVAL
        ));
    }

    #[test]
    fn does_not_trigger_without_new_speech() {
        let s = SchedulerTick {
            cursor_ms: Some(20_000),
            ..tick()
        };
        assert!(!should_trigger(&s, INTERVAL));
        let s = SchedulerTick {
            latest_final_ms: None,
            ..tick()
        };
        assert!(!should_trigger(&s, INTERVAL));
    }

    #[test]
    fn waits_for_the_interval_to_complete() {
        let s = SchedulerTick {
            elapsed_ms: 300,
            ..tick()
        };
        assert!(!should_trigger(&s, INTERVAL));
        let s = SchedulerTick {
            elapsed_ms: INTERVAL,
            ..tick()
        };
        assert!(should_trigger(&s, INTERVAL));
    }

    #[test]
    fn opening_the_window_restarts_the_countdown() {
        // A janela acabou de abrir: mesmo com fala antiga, o ciclo recomeça e
        // não dispara (a janela não gera ao abrir).
        let s = SchedulerTick {
            elapsed_ms: 0,
            ..tick()
        };
        assert!(!should_trigger(&s, INTERVAL));
        let c = cadence(&s, INTERVAL);
        assert_eq!(c.remaining_ms, INTERVAL);
        assert!(!c.awaiting_speech);

        // Depois do intervalo, a sugestão vem.
        let s = SchedulerTick {
            elapsed_ms: INTERVAL,
            ..s
        };
        assert!(should_trigger(&s, INTERVAL));
    }

    // --- Cadência (BR-039) -------------------------------------------------

    #[test]
    fn cadence_counts_down_the_interval() {
        let s = SchedulerTick {
            elapsed_ms: 1_000,
            ..tick()
        };
        let c = cadence(&s, INTERVAL);
        assert!(c.enabled && c.running);
        assert!(!c.awaiting_speech);
        assert_eq!(c.remaining_ms, 19_000);
        assert_eq!(c.window_ms, INTERVAL);
    }

    #[test]
    fn cadence_does_not_restart_on_speech() {
        // Duas falas em instantes diferentes, o mesmo ciclo: o contador segue
        // o tempo, não a fala (fala nova só libera a geração no fim).
        let early = SchedulerTick {
            elapsed_ms: 2_000,
            latest_final_ms: Some(30_000),
            ..tick()
        };
        let later = SchedulerTick {
            elapsed_ms: 2_000,
            latest_final_ms: Some(31_000),
            ..tick()
        };
        assert_eq!(cadence(&early, INTERVAL).remaining_ms, 18_000);
        assert_eq!(cadence(&later, INTERVAL).remaining_ms, 18_000);
    }

    #[test]
    fn cadence_counts_from_the_window_open_without_speech() {
        // Sem fala, a janela abriu agora: conta o intervalo cheio.
        let s = SchedulerTick {
            cursor_ms: Some(20_000),
            elapsed_ms: 2_000,
            ..tick()
        };
        let c = cadence(&s, INTERVAL);
        assert_eq!(c.remaining_ms, 18_000);
        assert!(!c.awaiting_speech);
    }

    #[test]
    fn cadence_reports_awaiting_speech_without_new_content() {
        let s = SchedulerTick {
            cursor_ms: Some(20_000),
            ..tick()
        };
        let c = cadence(&s, INTERVAL);
        assert!(c.awaiting_speech);
        assert_eq!(c.remaining_ms, 0);
        assert!(c.running);
    }

    #[test]
    fn cadence_triggers_as_soon_as_new_speech_arrives_after_the_interval() {
        let awaiting = SchedulerTick {
            cursor_ms: Some(20_000),
            ..tick()
        };
        assert!(cadence(&awaiting, INTERVAL).awaiting_speech);

        // Fala nova depois do intervalo: gera no próximo tick, sem esperar
        // outro intervalo cheio.
        let fresh = SchedulerTick {
            cursor_ms: Some(20_000),
            latest_final_ms: Some(21_000),
            ..tick()
        };
        let c = cadence(&fresh, INTERVAL);
        assert!(c.running);
        assert!(!c.awaiting_speech);
        assert_eq!(c.remaining_ms, 0);
        assert!(should_trigger(&fresh, INTERVAL));
    }

    #[test]
    fn cadence_is_paused_when_the_automatic_is_off() {
        let s = SchedulerTick {
            enabled: false,
            ..tick()
        };
        let c = cadence(&s, INTERVAL);
        assert!(!c.enabled);
        assert!(!c.running);
        assert_eq!(c.remaining_ms, 0);
        assert!(!c.awaiting_speech);
    }

    #[test]
    fn cadence_is_idle_without_a_session() {
        let s = SchedulerTick {
            has_session: false,
            ..tick()
        };
        let c = cadence(&s, INTERVAL);
        assert!(!c.running);
        assert_eq!(c.remaining_ms, 0);
    }
}
