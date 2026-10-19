//! Decisão **pura** de quando gerar uma sugestão automática.
//!
//! Transposto do Perssua: os insights automáticos não dependem de uma janela
//! aberta — eles reagem a **fala nova**. O intervalo configurado
//! (`suggestion_interval_secs`) é o **tempo de silêncio** depois da última fala:
//! a cada fala nova o contador **reinicia**; quando o silêncio completa o
//! intervalo, a sugestão é gerada (BR-039). Assim uma sequência de perguntas
//! próximas vira **uma** sugestão, não uma por balão.
//!
//! **Abrir a janela também reinicia o contador** (pedido do usuário): a janela
//! não gera ao abrir; o auto começa a contar do intervalo. Sem fala nova desde a
//! última sugestão, o contador para em 0 e espera.
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
    /// Tempo desde o início da última fala finalizada (ms).
    pub ms_since_latest_final: i64,
    /// Tempo desde que a janela de Sugestões abriu (ms).
    ///
    /// `i64::MAX` quando a janela está fechada (sem geração).
    pub ms_since_window_open: i64,
}

impl SchedulerTick {
    /// Silêncio efetivo: o mais recente entre a fala e a abertura da janela.
    ///
    /// Abrir a janela **reinicia a contagem** (a janela não gera ao abrir).
    fn silence_elapsed_ms(&self) -> i64 {
        self.ms_since_latest_final
            .min(self.ms_since_window_open)
            .max(0)
    }
}

/// Decide se é hora de gerar uma sugestão automática.
///
/// Regras (todas precisam valer):
/// - ligado e com sessão aberta;
/// - existe fala finalizada **depois** do cursor (há conteúdo novo);
/// - o silêncio (fala **ou** abertura da janela) completou `silence_ms`.
///
/// Uma fala nova — e a abertura da janela — **reinicia** o silêncio.
pub fn should_trigger(tick: &SchedulerTick, silence_ms: i64) -> bool {
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
    tick.silence_elapsed_ms() >= silence_ms
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
    /// Milissegundos até a próxima sugestão (o silêncio que falta).
    pub remaining_ms: i64,
    /// O silêncio venceu, mas não há fala nova desde a última sugestão.
    pub awaiting_speech: bool,
    /// Duração total do ciclo (desenha o anel de progresso).
    pub window_ms: i64,
}

/// Calcula a cadência a partir de um tick do agendador.
pub fn cadence(tick: &SchedulerTick, silence_ms: i64) -> Cadence {
    let window_ms = silence_ms.max(1);
    if !tick.enabled || !tick.has_session {
        return Cadence {
            enabled: tick.enabled,
            running: false,
            remaining_ms: 0,
            awaiting_speech: false,
            window_ms,
        };
    }

    // Fala nova (ou a janela recém-aberta): o contador corre.
    let remaining = silence_ms.saturating_sub(tick.silence_elapsed_ms()).max(0);
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

    // Silêncio vencido com fala nova: a geração dispara neste tick.
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
            latest_final_ms: Some(10_000),
            // Silêncio já completo (o timer de 20 s venceu).
            ms_since_latest_final: 25_000,
            // Janela aberta há muito tempo.
            ms_since_window_open: 120_000,
        }
    }

    const SILENCE: i64 = 20_000;

    #[test]
    fn triggers_with_new_speech_after_the_silence() {
        assert!(should_trigger(&tick(), SILENCE));
    }

    #[test]
    fn does_not_trigger_without_session_or_when_disabled() {
        assert!(!should_trigger(
            &SchedulerTick {
                enabled: false,
                ..tick()
            },
            SILENCE
        ));
        assert!(!should_trigger(
            &SchedulerTick {
                has_session: false,
                ..tick()
            },
            SILENCE
        ));
    }

    #[test]
    fn does_not_trigger_without_new_speech() {
        let s = SchedulerTick {
            cursor_ms: Some(10_000),
            ..tick()
        };
        assert!(!should_trigger(&s, SILENCE));
        let s = SchedulerTick {
            latest_final_ms: None,
            ..tick()
        };
        assert!(!should_trigger(&s, SILENCE));
    }

    #[test]
    fn waits_for_the_silence_to_complete() {
        let s = SchedulerTick {
            ms_since_latest_final: 300,
            ..tick()
        };
        assert!(!should_trigger(&s, SILENCE));
        let s = SchedulerTick {
            ms_since_latest_final: SILENCE,
            ..tick()
        };
        assert!(should_trigger(&s, SILENCE));
    }

    #[test]
    fn opening_the_window_restarts_the_countdown() {
        // A janela acabou de abrir: mesmo com fala antiga, a contagem reinicia
        // e não dispara (a janela não gera ao abrir).
        let s = SchedulerTick {
            ms_since_latest_final: 60_000,
            ms_since_window_open: 0,
            ..tick()
        };
        assert!(!should_trigger(&s, SILENCE));
        let c = cadence(&s, SILENCE);
        assert_eq!(c.remaining_ms, SILENCE);
        assert!(!c.awaiting_speech);

        // Depois do intervalo, a sugestão vem.
        let s = SchedulerTick {
            ms_since_window_open: SILENCE,
            ..s
        };
        assert!(should_trigger(&s, SILENCE));
    }

    // --- Cadência (BR-039) -------------------------------------------------

    #[test]
    fn cadence_counts_down_the_silence() {
        let s = SchedulerTick {
            ms_since_latest_final: 1_000,
            ..tick()
        };
        let c = cadence(&s, SILENCE);
        assert!(c.enabled && c.running);
        assert!(!c.awaiting_speech);
        assert_eq!(c.remaining_ms, 19_000);
        assert_eq!(c.window_ms, SILENCE);
    }

    #[test]
    fn cadence_restarts_on_each_new_speech() {
        assert_eq!(
            cadence(
                &SchedulerTick {
                    ms_since_latest_final: 2_000,
                    ..tick()
                },
                SILENCE
            )
            .remaining_ms,
            18_000
        );
        assert_eq!(
            cadence(
                &SchedulerTick {
                    ms_since_latest_final: 200,
                    ..tick()
                },
                SILENCE
            )
            .remaining_ms,
            19_800
        );
    }

    #[test]
    fn cadence_counts_from_the_window_open_without_speech() {
        // Sem fala (a última é antiga), a janela abriu agora: conta 20 s.
        let s = SchedulerTick {
            cursor_ms: Some(10_000),
            ms_since_latest_final: 60_000,
            ms_since_window_open: 2_000,
            ..tick()
        };
        let c = cadence(&s, SILENCE);
        assert_eq!(c.remaining_ms, 18_000);
        assert!(!c.awaiting_speech);
    }

    #[test]
    fn cadence_reports_awaiting_speech_without_new_content() {
        let s = SchedulerTick {
            cursor_ms: Some(10_000),
            ..tick()
        };
        let c = cadence(&s, SILENCE);
        assert!(c.awaiting_speech);
        assert_eq!(c.remaining_ms, 0);
        assert!(c.running);
    }

    #[test]
    fn cadence_is_paused_when_the_automatic_is_off() {
        let s = SchedulerTick {
            enabled: false,
            ..tick()
        };
        let c = cadence(&s, SILENCE);
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
        let c = cadence(&s, SILENCE);
        assert!(!c.running);
        assert_eq!(c.remaining_ms, 0);
    }
}
