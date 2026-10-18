//! Monitor atrasado do microfone sobre a API simple do PulseAudio/PipeWire.
//!
//! O servidor faz o remix/resample para a saída padrão, então o cliente só
//! escreve PCM 16 bits mono na taxa de trabalho (`TARGET_SAMPLE_RATE`).
//!
//! Fluxo: `push` enfileira o bloco com carimbo de tempo
//! ([`DelayedAudioQueue`]); uma thread escreve no sink o que já venceu — e
//! **silêncio** quando nada venceu, para o stream não secar (nem ficar com
//! atraso crescente). Nunca logamos amostras (regra do projeto).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;

use crate::domain::audio::{DelayedAudioQueue, DEFAULT_MONITOR_DELAY_MS, TARGET_SAMPLE_RATE};
use crate::error::{GhostWhisperError, Result};
use crate::ports::audio_monitor::AudioMonitor;

/// Variável de ambiente do atraso padrão do monitor (ms).
pub const MONITOR_DELAY_ENV: &str = "GHOSTWHISPER_MONITOR_DELAY_MS";

/// Silêncio injetado quando não há áudio pronto (ms).
const SILENCE_STEP_MS: u64 = 20;
/// Espera máxima pela abertura do sink (a UI recebe erro em vez de silêncio).
const OPEN_TIMEOUT: Duration = Duration::from_secs(3);

/// Resolve o atraso padrão do monitor (override por ambiente).
pub fn default_delay_ms() -> u64 {
    std::env::var(MONITOR_DELAY_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_MONITOR_DELAY_MS)
}

/// Monitor atrasado do microfone (PulseAudio/PipeWire).
pub struct PulseDelayedMonitor {
    /// Aberto e escrevendo (o `push` só enfileira quando ligado).
    running: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
    queue: Arc<Mutex<DelayedAudioQueue>>,
    /// Origem do relógio monotônico dos carimbos de tempo.
    origin: Arc<Mutex<Instant>>,
}

impl Default for PulseDelayedMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl PulseDelayedMonitor {
    /// Cria o monitor (nada é reproduzido até `start`).
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(true)),
            thread: Mutex::new(None),
            queue: Arc::new(Mutex::new(DelayedAudioQueue::new(DEFAULT_MONITOR_DELAY_MS))),
            origin: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Milissegundos desde o início da reprodução atual.
    fn now_ms(&self) -> u64 {
        self.origin
            .lock()
            .map(|origin| origin.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }
}

impl AudioMonitor for PulseDelayedMonitor {
    fn start(&self, delay_ms: u64) -> Result<()> {
        self.stop()?;

        {
            let mut queue = self
                .queue
                .lock()
                .map_err(|_| GhostWhisperError::Other("fila do monitor envenenada".to_string()))?;
            *queue = DelayedAudioQueue::new(delay_ms);
        }
        if let Ok(mut origin) = self.origin.lock() {
            *origin = Instant::now();
        }
        let delay_ms = self
            .queue
            .lock()
            .map(|queue| queue.delay_ms())
            .unwrap_or(delay_ms);

        self.stop.store(false, Ordering::Relaxed);
        let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();

        let queue = Arc::clone(&self.queue);
        let origin = Arc::clone(&self.origin);
        let stop = Arc::clone(&self.stop);
        let running = Arc::clone(&self.running);
        let handle = thread::Builder::new()
            .name("mic-monitor".into())
            .spawn(move || {
                if let Err(error) = run_monitor(&queue, &origin, &stop, ready_tx) {
                    tracing::warn!("monitor do microfone encerrado: {error}");
                }
                running.store(false, Ordering::Relaxed);
            })
            .map_err(|error| {
                GhostWhisperError::Other(format!("falha ao iniciar o monitor: {error}"))
            })?;

        // A abertura do sink acontece na thread (o `Simple` não precisa ser Send),
        // mas o erro volta para quem chamou: nada de "ligado" sem som.
        match ready_rx.recv_timeout(OPEN_TIMEOUT) {
            Ok(Ok(())) => {
                self.running.store(true, Ordering::Relaxed);
                if let Ok(mut guard) = self.thread.lock() {
                    *guard = Some(handle);
                }
                tracing::info!(delay_ms, "monitor do microfone ligado (use fone de ouvido)");
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = handle.join();
                Err(error)
            }
            Err(_) => {
                // Sem confirmação: mantém ligado (o stream pode ter aberto devagar),
                // mas registra a incerteza.
                tracing::warn!("monitor não confirmou a abertura em {OPEN_TIMEOUT:?}");
                self.running.store(true, Ordering::Relaxed);
                if let Ok(mut guard) = self.thread.lock() {
                    *guard = Some(handle);
                }
                Ok(())
            }
        }
    }

    fn push(&self, samples: &[f32]) {
        if !self.running.load(Ordering::Relaxed) {
            return;
        }
        let now_ms = self.now_ms();
        if let Ok(mut queue) = self.queue.lock() {
            queue.push(samples, now_ms);
        }
    }

    fn stop(&self) -> Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        self.running.store(false, Ordering::Relaxed);
        if let Ok(mut guard) = self.thread.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
        if let Ok(mut queue) = self.queue.lock() {
            let dropped = queue.dropped_samples();
            let pending = queue.pending_samples();
            if dropped > 0 || pending > 0 {
                tracing::debug!(
                    dropped,
                    pending,
                    "monitor encerrado com áudio descartado/pendente"
                );
            }
            queue.clear();
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

/// Abre a saída padrão e escreve o que estiver pronto até `stop`.
fn run_monitor(
    queue: &Arc<Mutex<DelayedAudioQueue>>,
    origin: &Arc<Mutex<Instant>>,
    stop: &Arc<AtomicBool>,
    ready: mpsc::Sender<Result<()>>,
) -> Result<()> {
    let spec = Spec {
        format: Format::S16le,
        channels: 1,
        rate: TARGET_SAMPLE_RATE,
    };

    let simple = match Simple::new(
        None,
        "GhostWhisper",
        Direction::Playback,
        None,
        "mic-monitor",
        &spec,
        None,
        None,
    ) {
        Ok(simple) => simple,
        Err(error) => {
            let error = GhostWhisperError::Other(format!(
                "falha ao abrir a saída de áudio do monitor: {error}"
            ));
            let _ = ready.send(Err(GhostWhisperError::Other(error.to_string())));
            return Err(error);
        }
    };
    let _ = ready.send(Ok(()));

    let silence = vec![0u8; (TARGET_SAMPLE_RATE as usize * SILENCE_STEP_MS as usize / 1000) * 2];
    let mut scratch: Vec<f32> = Vec::new();
    let mut bytes: Vec<u8> = Vec::new();

    while !stop.load(Ordering::Relaxed) {
        let now_ms = origin
            .lock()
            .map(|origin| origin.elapsed().as_millis() as u64)
            .unwrap_or(0);

        scratch.clear();
        match queue.lock() {
            Ok(mut queue) => queue.drain_ready(now_ms, &mut scratch),
            Err(_) => break,
        }

        if scratch.is_empty() {
            // Mantém o stream vivo e no ritmo do relógio de áudio.
            if let Err(error) = simple.write(&silence) {
                return Err(GhostWhisperError::Other(format!(
                    "monitor falhou ao escrever silêncio: {error}"
                )));
            }
            continue;
        }

        bytes.clear();
        bytes.reserve(scratch.len() * 2);
        for sample in &scratch {
            // O sinal já vem no nível do ASR (ganho + gate, BR-018): o monitor
            // reproduz o que o whisper recebe. A saturação protege a saída.
            let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        if let Err(error) = simple.write(&bytes) {
            return Err(GhostWhisperError::Other(format!(
                "monitor falhou ao escrever áudio: {error}"
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_delay_falls_back_to_the_constant() {
        // Sem ambiente definido, usa o padrão do domínio.
        if std::env::var(MONITOR_DELAY_ENV).is_err() {
            assert_eq!(default_delay_ms(), DEFAULT_MONITOR_DELAY_MS);
        }
    }

    #[test]
    fn monitor_is_idle_until_started() {
        let monitor = PulseDelayedMonitor::new();
        assert!(!monitor.is_running());
        // `push` desligado não enfileira nada (e não abre saída de áudio).
        monitor.push(&[0.5; 160]);
        assert_eq!(monitor.queue.lock().expect("queue").pending_samples(), 0);

        monitor.stop().expect("stop sem start");
        assert!(!monitor.is_running());
    }

    #[test]
    #[ignore = "requer servidor Pulse/PipeWire e dispositivo de saída"]
    fn plays_and_stops_against_a_real_server() {
        let monitor = PulseDelayedMonitor::new();
        monitor.start(50).expect("start");
        assert!(monitor.is_running());

        monitor.push(&vec![0.0; TARGET_SAMPLE_RATE as usize / 10]);
        thread::sleep(Duration::from_millis(300));

        monitor.stop().expect("stop");
        assert!(!monitor.is_running());
        assert_eq!(monitor.queue.lock().expect("queue").pending_samples(), 0);
    }
}
