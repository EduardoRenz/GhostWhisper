//! Fila global do ASR com **prioridade para o passe final**.
//!
//! Só uma decodificação do Whisper roda por vez (um contexto é compartilhado
//! entre os canais). Sem prioridade, o passe final de um canal entra atrás da
//! prévia do outro — e a frase que o usuário acabou de falar espera por um texto
//! rolante que será substituído de qualquer forma.
//!
//! Aqui o `Final` fura a fila das prévias. Não há preempção: uma prévia **em
//! execução** termina (o whisper não interrompe um passe), mas nenhuma prévia
//! que ainda está **esperando** passa na frente de um final pendente.

use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::domain::transcription::SpeechQuality;
use crate::error::{GhostWhisperError, Result};

/// Estado da fila: ocupada, e quantos de cada prioridade esperam.
#[derive(Debug, Default)]
struct GateState {
    busy: bool,
    waiting_final: usize,
    waiting_preview: usize,
}

/// Trava global do ASR com prioridade.
#[derive(Debug, Default)]
pub struct SpeechGate {
    state: Mutex<GateState>,
    cv: Condvar,
}

impl SpeechGate {
    /// Cria a trava; deve ser compartilhada entre todos os canais.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adquire a trava; `Final` passa na frente de prévias que estão esperando.
    ///
    /// `timeout` limita a espera (o pipeline não pode travar para sempre se um
    /// passe ficar preso). Devolve erro se o prazo estourar.
    pub(crate) fn acquire(
        &self,
        quality: SpeechQuality,
        timeout: Duration,
    ) -> Result<SpeechGateGuard<'_>> {
        let deadline = Instant::now() + timeout;
        let is_final = matches!(quality, SpeechQuality::Final);
        let mut state = self
            .state
            .lock()
            .map_err(|_| GhostWhisperError::Other("fila do ASR envenenada".into()))?;

        if is_final {
            state.waiting_final += 1;
        } else {
            state.waiting_preview += 1;
        }

        loop {
            // Prévia cede a vez a qualquer final que esteja esperando.
            let blocked = state.busy || (!is_final && state.waiting_final > 0);
            if !blocked {
                state.busy = true;
                if is_final {
                    state.waiting_final -= 1;
                } else {
                    state.waiting_preview -= 1;
                }
                return Ok(SpeechGateGuard { gate: self });
            }

            let now = Instant::now();
            if now >= deadline {
                if is_final {
                    state.waiting_final -= 1;
                } else {
                    state.waiting_preview -= 1;
                }
                return Err(GhostWhisperError::Other(
                    "fila do ASR ocupada demais (espera >20s)".into(),
                ));
            }

            let (next, _) = self
                .cv
                .wait_timeout(state, deadline - now)
                .map_err(|_| GhostWhisperError::Other("fila do ASR envenenada".into()))?;
            state = next;
        }
    }

    /// Quantos finais e prévias esperam (diagnóstico e testes).
    #[cfg(test)]
    fn waiting(&self) -> (usize, usize) {
        self.state
            .lock()
            .map(|state| (state.waiting_final, state.waiting_preview))
            .unwrap_or((0, 0))
    }

    fn release(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.busy = false;
            // Acorda todos: uma prévia pode ter sido acordada primeiro e voltar
            // a dormir ao ver um final na fila.
            self.cv.notify_all();
        }
    }
}

/// Posse da trava enquanto a decodificação roda.
#[derive(Debug)]
pub(crate) struct SpeechGateGuard<'a> {
    gate: &'a SpeechGate,
}

impl Drop for SpeechGateGuard<'_> {
    fn drop(&mut self) {
        self.gate.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;

    /// Espera (com prazo) uma condição observável da fila.
    fn wait_until(mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if condition() {
                return;
            }
            thread::sleep(Duration::from_millis(2));
        }
        panic!("condição não foi satisfeita a tempo");
    }

    /// Roda `acquire`, registra a ordem e libera.
    fn contender(
        gate: Arc<SpeechGate>,
        quality: SpeechQuality,
        order: Arc<Mutex<Vec<&'static str>>>,
        label: &'static str,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let _guard = gate
                .acquire(quality, Duration::from_secs(5))
                .expect("adquirir");
            order.lock().expect("ordem").push(label);
        })
    }

    #[test]
    fn final_overtakes_a_waiting_preview() {
        let gate = Arc::new(SpeechGate::new());
        let order = Arc::new(Mutex::new(Vec::new()));

        // Trava ocupada segura os dois concorrentes.
        let holder = gate
            .acquire(SpeechQuality::Preview, Duration::from_secs(1))
            .expect("holder");

        let preview = contender(
            Arc::clone(&gate),
            SpeechQuality::Preview,
            Arc::clone(&order),
            "preview",
        );
        wait_until(|| gate.waiting() == (0, 1));

        let fin = contender(
            Arc::clone(&gate),
            SpeechQuality::Final,
            Arc::clone(&order),
            "final",
        );
        wait_until(|| gate.waiting() == (1, 1));

        drop(holder);
        preview.join().expect("preview");
        fin.join().expect("final");

        assert_eq!(
            *order.lock().expect("ordem"),
            vec!["final", "preview"],
            "o final precisa passar na frente da prévia que espera"
        );
    }

    #[test]
    fn preview_runs_when_no_final_waits() {
        let gate = SpeechGate::new();
        let guard = gate
            .acquire(SpeechQuality::Preview, Duration::from_secs(1))
            .expect("prévia");
        drop(guard);
        assert!(gate
            .acquire(SpeechQuality::Final, Duration::from_secs(1))
            .is_ok());
    }

    #[test]
    fn queue_timeout_does_not_leave_a_stale_waiter() {
        let gate = Arc::new(SpeechGate::new());
        let holder = gate
            .acquire(SpeechQuality::Final, Duration::from_secs(1))
            .expect("holder");

        let started = Instant::now();
        let error = gate
            .acquire(SpeechQuality::Preview, Duration::from_millis(30))
            .expect_err("devia estourar o prazo");
        assert!(error.to_string().contains("ocupada demais"));
        assert!(started.elapsed() >= Duration::from_millis(30));
        assert_eq!(gate.waiting(), (0, 0), "contador não pode ficar preso");

        drop(holder);
    }

    #[test]
    fn waiters_are_woken_when_the_gate_is_released() {
        let gate = Arc::new(SpeechGate::new());
        let (tx, rx) = mpsc::channel();
        let holder = gate
            .acquire(SpeechQuality::Final, Duration::from_secs(1))
            .expect("holder");

        let waiter = {
            let gate = Arc::clone(&gate);
            thread::spawn(move || {
                let _guard = gate
                    .acquire(SpeechQuality::Preview, Duration::from_secs(2))
                    .expect("adquirir");
                tx.send(()).expect("sinalizar");
            })
        };
        wait_until(|| gate.waiting() == (0, 1));
        drop(holder);
        rx.recv_timeout(Duration::from_secs(2))
            .expect("a prévia devia ser acordada");
        waiter.join().expect("waiter");
    }
}
