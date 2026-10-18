//! Decisão pura de **quando** pré-visualizar e **quando** fechar o turno.
//!
//! Vive separado do pipeline para ser testável sem threads, sem relógio e sem
//! áudio (R4/§1: `application` testável, sem I/O). O pipeline apenas mantém o
//! estado (buffers, instantes) e executa a decisão.
//!
//! Transposto do Perssua (`WhisperSessionStreaming.js` + `WhisperSession.js`
//! `appendAudio`), que é a referência de produto (BR-015):
//! - prévia quando há ~`preview_step_ms` de **áudio novo** desde o último passe
//!   (`SERVER_WINDOW_DURATION_SEC = 0.75`);
//! - primeira prévia só depois de `initial_preview_ms` (`INITIAL_PREVIEW_MS`);
//! - força um passe se ficar `idle_force_ms` sem processar
//!   (`SERVER_IDLE_FORCE_PROCESS_MS`);
//! - fecha o turno após `silence_ms` de silêncio (`VAD_SILENCE_DURATION_MS`).

/// Limiares de tempo do turno.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnTimings {
    /// Primeira prévia só depois disto de turno (`INITIAL_PREVIEW_MS = 1200`).
    pub initial_preview_ms: i64,
    /// Passo mínimo de áudio novo entre prévias (`0.75 s`).
    pub preview_step_ms: i64,
    /// Força uma prévia quando nada é processado por este tempo (`1 s`).
    pub idle_force_ms: i64,
    /// Silêncio que encerra o turno (`1 s`).
    pub silence_ms: i64,
    /// Fala mínima do turno para virar segmento final (`0.8 s`).
    pub min_segment_ms: i64,
    /// Áudio mínimo do turno para valer uma decodificação (`0.35 s`).
    pub min_window_ms: i64,
    /// Teto do turno em fala contínua.
    ///
    /// O Perssua usa `MAX_CONTEXT_MS = 55 s`, mas a medição neste projeto
    /// (`benchmark_rtf_by_audio_duration`) mostrou um **penhasco de ~10× por
    /// chamada a partir de 30 s** (o encoder do whisper só enxerga 30 s). Cortar
    /// abaixo disso mantém toda decodificação na faixa barata.
    pub max_turn_ms: i64,
}

impl Default for TurnTimings {
    fn default() -> Self {
        Self {
            initial_preview_ms: 1_200,
            preview_step_ms: 750,
            idle_force_ms: 1_000,
            silence_ms: 1_000,
            min_segment_ms: 800,
            min_window_ms: 350,
            max_turn_ms: 25_000,
        }
    }
}

impl TurnTimings {
    /// Limiares com override por ambiente (permite afinar na máquina do usuário
    /// sem recompilar, como o `runtimeConfig.js` do Perssua).
    pub fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            initial_preview_ms: env_ms(
                "GHOSTWHISPER_INITIAL_PREVIEW_MS",
                defaults.initial_preview_ms,
            ),
            preview_step_ms: env_ms("GHOSTWHISPER_PREVIEW_STEP_MS", defaults.preview_step_ms),
            idle_force_ms: env_ms("GHOSTWHISPER_IDLE_FORCE_MS", defaults.idle_force_ms),
            silence_ms: env_ms("GHOSTWHISPER_VAD_SILENCE_MS", defaults.silence_ms),
            min_segment_ms: env_ms("GHOSTWHISPER_MIN_SEGMENT_MS", defaults.min_segment_ms),
            min_window_ms: env_ms("GHOSTWHISPER_MIN_WINDOW_MS", defaults.min_window_ms),
            max_turn_ms: env_ms("GHOSTWHISPER_MAX_TURN_MS", defaults.max_turn_ms),
        }
    }
}

/// Lê uma duração (ms) do ambiente; valor ausente/inválido usa o padrão.
fn env_ms(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

/// Observação do turno num instante (entradas da decisão).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnState {
    /// Áudio acumulado no turno (ms).
    pub buffered_ms: i64,
    /// Houve fala no turno.
    pub had_speech: bool,
    /// Fim (ms, relativo ao turno) da última janela com fala.
    pub last_voiced_end_ms: i64,
    /// Comprimento do buffer (ms) na última decodificação **bem-sucedida**.
    pub last_decode_ms: i64,
    /// Tempo decorrido desde a última **tentativa** de decodificação.
    pub ms_since_last_attempt: i64,
    /// Há uma decodificação em andamento.
    pub decode_in_flight: bool,
    /// Já foi pedido o passe final do turno atual.
    pub final_requested: bool,
    /// A última tentativa de decodificação falhou (transitório).
    pub last_attempt_failed: bool,
}

/// O que fazer neste instante.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Nada a fazer (aguarda mais áudio).
    Wait,
    /// Decodifica o turno que cresce e publica uma prévia revisável.
    Preview {
        /// Milissegundos do turno a decodificar.
        cut_ms: i64,
    },
    /// Fecha o turno com um passe autoritativo.
    Finalize {
        /// Milissegundos do turno a decodificar.
        cut_ms: i64,
    },
}

/// Decide o próximo passo do turno.
pub fn decide(state: &TurnState, timings: &TurnTimings) -> Decision {
    // Teto de fala contínua: fecha cortando exatamente no teto.
    if state.buffered_ms >= timings.max_turn_ms {
        return Decision::Finalize {
            cut_ms: timings.max_turn_ms,
        };
    }

    // Silêncio suficiente após fala: o turno terminou.
    let silence_ms = state.buffered_ms - state.last_voiced_end_ms;
    if state.had_speech
        && silence_ms >= timings.silence_ms
        && state.buffered_ms >= timings.min_segment_ms
    {
        return Decision::Finalize {
            cut_ms: state.buffered_ms,
        };
    }

    // Prévia durante a fala (texto rolante).
    if state.had_speech
        && state.buffered_ms >= timings.initial_preview_ms
        && state.buffered_ms >= timings.min_window_ms
        && !state.decode_in_flight
        && !state.final_requested
    {
        let step_ready = state.buffered_ms - state.last_decode_ms >= timings.preview_step_ms;
        let idle_ready = state.ms_since_last_attempt >= timings.idle_force_ms;
        // Após uma falha transitória, espera o relógio de ociosidade: sem isso o
        // agendamento re-tentaria a cada tick (50 ms) enquanto o ASR estivesse
        // ocupado, gastando CPU e log à toa.
        let ready = if state.last_attempt_failed {
            idle_ready
        } else {
            step_ready || idle_ready
        };
        if ready {
            return Decision::Preview {
                cut_ms: state.buffered_ms,
            };
        }
    }

    Decision::Wait
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(buffered_ms: i64) -> TurnState {
        TurnState {
            buffered_ms,
            had_speech: true,
            last_voiced_end_ms: buffered_ms,
            last_decode_ms: 0,
            ms_since_last_attempt: 0,
            decode_in_flight: false,
            final_requested: false,
            last_attempt_failed: false,
        }
    }

    #[test]
    fn waits_before_the_initial_window() {
        let timings = TurnTimings::default();
        // 1,0 s: abaixo de INITIAL_PREVIEW_MS (1,2 s) ⇒ nada.
        assert_eq!(decide(&state(1_000), &timings), Decision::Wait);
        // Cruza o limiar ⇒ primeira prévia.
        assert_eq!(
            decide(&state(1_250), &timings),
            Decision::Preview { cut_ms: 1_250 }
        );
    }

    #[test]
    fn waits_without_speech() {
        let timings = TurnTimings::default();
        let mut silent = state(5_000);
        silent.had_speech = false;
        assert_eq!(decide(&silent, &timings), Decision::Wait);
    }

    #[test]
    fn preview_respects_the_step() {
        let timings = TurnTimings::default();
        let mut s = state(1_500);
        s.last_decode_ms = 1_500;
        // Nada novo desde a última decodificação ⇒ aguarda.
        assert_eq!(decide(&s, &timings), Decision::Wait);
        // +0,75 s de áudio novo ⇒ nova prévia.
        assert_eq!(
            decide(&state(2_250), &timings),
            Decision::Preview { cut_ms: 2_250 }
        );
    }

    #[test]
    fn idle_force_triggers_a_preview_without_new_audio() {
        let timings = TurnTimings::default();
        let mut s = state(1_500);
        s.last_decode_ms = 1_500;
        s.ms_since_last_attempt = timings.idle_force_ms;
        assert_eq!(decide(&s, &timings), Decision::Preview { cut_ms: 1_500 });
    }

    #[test]
    fn in_flight_or_requested_final_suppresses_previews() {
        let timings = TurnTimings::default();
        let mut busy = state(3_000);
        busy.decode_in_flight = true;
        assert_eq!(decide(&busy, &timings), Decision::Wait);

        let mut final_pending = state(3_000);
        final_pending.final_requested = true;
        assert_eq!(decide(&final_pending, &timings), Decision::Wait);
    }

    #[test]
    fn failed_attempt_waits_for_the_idle_clock() {
        let timings = TurnTimings::default();
        let mut s = state(3_000);
        s.last_decode_ms = 1_500; // o passo já está satisfeito
        s.ms_since_last_attempt = 0;
        s.last_attempt_failed = true;
        // Não re-tenta só porque o passo está satisfeito.
        assert_eq!(decide(&s, &timings), Decision::Wait);

        s.ms_since_last_attempt = timings.idle_force_ms;
        assert_eq!(decide(&s, &timings), Decision::Preview { cut_ms: 3_000 });
    }

    #[test]
    fn silence_finalizes_the_turn() {
        let timings = TurnTimings::default();
        let mut s = state(2_000);
        s.last_voiced_end_ms = 900; // 1,1 s de silêncio
        assert_eq!(decide(&s, &timings), Decision::Finalize { cut_ms: 2_000 });
    }

    #[test]
    fn short_turn_after_silence_is_not_finalized() {
        let timings = TurnTimings::default();
        // 0,5 s < MIN_SEGMENT_MS (0,8 s): ruído curto não vira turno.
        let mut s = state(500);
        s.last_voiced_end_ms = 0;
        assert_eq!(decide(&s, &timings), Decision::Wait);
    }

    #[test]
    fn continuous_speech_is_cut_at_the_turn_cap() {
        let timings = TurnTimings::default();
        let s = state(timings.max_turn_ms);
        assert_eq!(
            decide(&s, &timings),
            Decision::Finalize {
                cut_ms: timings.max_turn_ms
            }
        );
    }

    #[test]
    fn default_turn_cap_stays_below_the_whisper_window() {
        // 30 s é onde o custo por chamada explode (~10×); o teto precisa ficar
        // abaixo disso.
        assert!(TurnTimings::default().max_turn_ms < 30_000);
    }

    #[test]
    fn env_override_parses_and_rejects_garbage() {
        // Regra pura: usa `env_ms` como está exposta.
        std::env::remove_var("GHOSTWHISPER_TEST_MS");
        assert_eq!(env_ms("GHOSTWHISPER_TEST_MS", 42), 42);

        std::env::set_var("GHOSTWHISPER_TEST_MS", "1500");
        assert_eq!(env_ms("GHOSTWHISPER_TEST_MS", 42), 1_500);

        std::env::set_var("GHOSTWHISPER_TEST_MS", "0");
        assert_eq!(env_ms("GHOSTWHISPER_TEST_MS", 42), 42);

        std::env::set_var("GHOSTWHISPER_TEST_MS", "abc");
        assert_eq!(env_ms("GHOSTWHISPER_TEST_MS", 42), 42);
        std::env::remove_var("GHOSTWHISPER_TEST_MS");
    }
}
