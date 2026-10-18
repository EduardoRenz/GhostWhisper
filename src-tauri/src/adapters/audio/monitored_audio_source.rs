//! Fonte de áudio que **espelha** o que passa para o monitor atrasado.
//!
//! É um decorador sobre [`AudioSource`]: entrega as amostras para o consumidor
//! normal (o pipeline de transcrição) e, no mesmo ponto, para o
//! [`AudioMonitor`] — de modo que o usuário ouça **exatamente** o que o ASR
//! recebe (ganho e gate incluídos, BR-018).
//!
//! O monitor é opcional em tempo de execução: ligado só enquanto o usuário pede
//! ("ouvir minha voz" no teste de áudio), então numa reunião normal nada é
//! reproduzido.

use std::sync::Arc;

use crate::domain::audio::AudioDevice;
use crate::error::Result;
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::audio_monitor::AudioMonitor;
use crate::ports::audio_source::AudioSource;

/// Fonte de áudio com um ponto de escuta para o monitor.
pub struct MonitoredAudioSource {
    inner: Arc<dyn AudioSource>,
    monitor: Arc<dyn AudioMonitor>,
}

impl MonitoredAudioSource {
    /// Envolve uma fonte de áudio com o ponto de escuta do monitor.
    pub fn new(inner: Arc<dyn AudioSource>, monitor: Arc<dyn AudioMonitor>) -> Self {
        Self { inner, monitor }
    }
}

impl AudioSource for MonitoredAudioSource {
    fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        self.inner.list_devices()
    }

    fn start(&self, device_id: Option<String>, consumer: Arc<dyn AudioConsumer>) -> Result<()> {
        let fan_out: Arc<dyn AudioConsumer> = Arc::new(MonitorFanOut {
            inner: consumer,
            monitor: Arc::clone(&self.monitor),
        });
        self.inner.start(device_id, fan_out)
    }

    fn stop(&self) -> Result<()> {
        self.inner.stop()
    }

    fn is_running(&self) -> bool {
        self.inner.is_running()
    }
}

/// Consumidor que repassa o áudio e o espelha no monitor.
struct MonitorFanOut {
    inner: Arc<dyn AudioConsumer>,
    monitor: Arc<dyn AudioMonitor>,
}

impl AudioConsumer for MonitorFanOut {
    fn consume(&self, samples: &[f32]) {
        // O monitor ignora o `push` quando desligado.
        self.monitor.push(samples);
        self.inner.consume(samples);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// Monitor de teste: registra o que foi empilhado.
    struct FakeAudioMonitor {
        running: AtomicBool,
        pushed: Mutex<Vec<f32>>,
        starts: AtomicUsize,
        fail: bool,
    }
    impl FakeAudioMonitor {
        fn new(fail: bool) -> Self {
            Self {
                running: AtomicBool::new(false),
                pushed: Mutex::new(Vec::new()),
                starts: AtomicUsize::new(0),
                fail,
            }
        }

        fn pushed(&self) -> Vec<f32> {
            self.pushed.lock().expect("pushed").clone()
        }
    }
    impl AudioMonitor for FakeAudioMonitor {
        fn start(&self, _delay_ms: u64) -> Result<()> {
            if self.fail {
                return Err(crate::error::GhostWhisperError::Other(
                    "sem saída de áudio".to_string(),
                ));
            }
            self.starts.fetch_add(1, Ordering::Relaxed);
            self.running.store(true, Ordering::Relaxed);
            Ok(())
        }
        fn push(&self, samples: &[f32]) {
            if self.running.load(Ordering::Relaxed) {
                self.pushed
                    .lock()
                    .expect("pushed")
                    .extend_from_slice(samples);
            }
        }
        fn stop(&self) -> Result<()> {
            self.running.store(false, Ordering::Relaxed);
            Ok(())
        }
        fn is_running(&self) -> bool {
            self.running.load(Ordering::Relaxed)
        }
    }

    /// Fonte que entrega o consumidor ao teste.
    struct CapturingSource {
        last: Mutex<Option<Arc<dyn AudioConsumer>>>,
        running: AtomicBool,
    }
    impl CapturingSource {
        fn new() -> Self {
            Self {
                last: Mutex::new(None),
                running: AtomicBool::new(false),
            }
        }
        fn consumer(&self) -> Arc<dyn AudioConsumer> {
            self.last
                .lock()
                .expect("last")
                .clone()
                .expect("consumidor capturado")
        }
    }
    impl AudioSource for CapturingSource {
        fn list_devices(&self) -> Result<Vec<AudioDevice>> {
            Ok(vec![])
        }
        fn start(
            &self,
            _device_id: Option<String>,
            consumer: Arc<dyn AudioConsumer>,
        ) -> Result<()> {
            self.running.store(true, Ordering::Relaxed);
            if let Ok(mut slot) = self.last.lock() {
                *slot = Some(consumer);
            }
            Ok(())
        }
        fn stop(&self) -> Result<()> {
            self.running.store(false, Ordering::Relaxed);
            Ok(())
        }
        fn is_running(&self) -> bool {
            self.running.load(Ordering::Relaxed)
        }
    }

    /// Consumidor que registra o último bloco recebido.
    struct RecordingConsumer {
        last: Mutex<Vec<f32>>,
    }
    impl RecordingConsumer {
        fn new() -> Self {
            Self {
                last: Mutex::new(Vec::new()),
            }
        }
        fn last(&self) -> Vec<f32> {
            self.last.lock().expect("last").clone()
        }
    }
    impl AudioConsumer for RecordingConsumer {
        fn consume(&self, samples: &[f32]) {
            if let Ok(mut slot) = self.last.lock() {
                *slot = samples.to_vec();
            }
        }
    }

    #[test]
    fn forwards_to_the_pipeline_and_mirrors_to_the_monitor() {
        let inner = Arc::new(CapturingSource::new());
        let monitor = Arc::new(FakeAudioMonitor::new(false));
        let source = MonitoredAudioSource::new(
            Arc::clone(&inner) as Arc<dyn AudioSource>,
            Arc::clone(&monitor) as Arc<dyn AudioMonitor>,
        );
        let sink = Arc::new(RecordingConsumer::new());
        source
            .start(None, Arc::clone(&sink) as Arc<dyn AudioConsumer>)
            .expect("start");

        monitor.start(400).expect("monitor ligado");
        inner.consumer().consume(&[0.25, -0.5]);

        assert_eq!(sink.last(), vec![0.25, -0.5], "o pipeline recebe igual");
        assert_eq!(monitor.pushed(), vec![0.25, -0.5], "o monitor espelha");
    }

    #[test]
    fn does_not_mirror_while_the_monitor_is_off() {
        let inner = Arc::new(CapturingSource::new());
        let monitor = Arc::new(FakeAudioMonitor::new(false));
        let source = MonitoredAudioSource::new(
            Arc::clone(&inner) as Arc<dyn AudioSource>,
            Arc::clone(&monitor) as Arc<dyn AudioMonitor>,
        );
        let sink = Arc::new(RecordingConsumer::new());
        source
            .start(None, Arc::clone(&sink) as Arc<dyn AudioConsumer>)
            .expect("start");

        inner.consumer().consume(&[0.5]);

        assert_eq!(sink.last(), vec![0.5]);
        assert!(monitor.pushed().is_empty(), "desligado não reproduz nada");
    }

    #[test]
    fn delegates_devices_and_state() {
        let inner = Arc::new(CapturingSource::new());
        let monitor = Arc::new(FakeAudioMonitor::new(false));
        let source = MonitoredAudioSource::new(
            Arc::clone(&inner) as Arc<dyn AudioSource>,
            Arc::clone(&monitor) as Arc<dyn AudioMonitor>,
        );

        assert!(source.list_devices().expect("devices").is_empty());
        assert!(!source.is_running());
        source
            .start(
                None,
                Arc::new(RecordingConsumer::new()) as Arc<dyn AudioConsumer>,
            )
            .expect("start");
        assert!(source.is_running());
        source.stop().expect("stop");
        assert!(!source.is_running());
    }

    #[test]
    fn monitor_failure_is_reported_to_the_caller() {
        let monitor = FakeAudioMonitor::new(true);
        // A fonte em si não falha: quem reporta o erro do monitor é o IPC.
        assert!(monitor.start(400).is_err());
        assert!(!monitor.is_running());
    }
}
