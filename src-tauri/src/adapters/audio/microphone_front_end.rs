//! Estágio de entrada do microfone: **ganho** e **noise gate** antes do pipeline.
//!
//! Transposto do Perssua (renderer do local-whisper: `micGainNode` →
//! `audioNoiseGate` → PCM16 → `sendAudio`):
//! - ganho configurável (1×–12×, padrão 8×), aplicado **só no microfone**;
//! - noise gate que **silencia** (não descarta) abaixo do limiar, com *hangover*.
//!
//! Ordem importa: o limiar do gate é medido **depois** do ganho, como no Perssua.
//!
//! É um **decorador** sobre a porta [`AudioSource`]: quem o usa continua vendo um
//! `AudioSource` comum, e as configurações são lidas a cada `start()` (mudar o
//! valor vale para a próxima captura).

use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::domain::audio::{
    apply_gain, clamp_mic_gain, AudioDevice, NoiseGate, DEFAULT_NOISE_GATE_HANGOVER_MS,
    SPEECH_RMS_THRESHOLD,
};
use crate::error::Result;
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::audio_source::AudioSource;
use crate::ports::settings_store::SettingsStore;

/// Variável de ambiente do limiar do gate de ruído (0 desliga).
pub const NOISE_GATE_RMS_ENV: &str = "GHOSTWHISPER_MIC_NOISE_GATE_RMS";
/// Variável de ambiente do hangover do gate (ms).
pub const NOISE_GATE_HANGOVER_ENV: &str = "GHOSTWHISPER_MIC_NOISE_GATE_HANGOVER_MS";

/// Resolve (limiar, hangover) do gate a partir de overrides de ambiente.
///
/// Regra pura para poder ser testada sem mexer no ambiente do processo.
fn resolve_gate(rms_raw: Option<&str>, hangover_raw: Option<&str>) -> (f32, u64) {
    let threshold = rms_raw
        .and_then(|raw| raw.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(SPEECH_RMS_THRESHOLD);
    let hangover = hangover_raw
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_NOISE_GATE_HANGOVER_MS);
    (threshold, hangover)
}

/// Lê o gate do ambiente (o padrão preserva o limiar de fala do app).
fn gate_from_env() -> (f32, u64) {
    let rms = std::env::var(NOISE_GATE_RMS_ENV).ok();
    let hangover = std::env::var(NOISE_GATE_HANGOVER_ENV).ok();
    resolve_gate(rms.as_deref(), hangover.as_deref())
}

/// Fonte de áudio que aplica ganho e noise gate de microfone.
pub struct MicrophoneFrontEnd {
    inner: Arc<dyn AudioSource>,
    settings: Arc<dyn SettingsStore>,
}

impl MicrophoneFrontEnd {
    /// Envolve uma fonte com o estágio de entrada do microfone.
    pub fn new(inner: Arc<dyn AudioSource>, settings: Arc<dyn SettingsStore>) -> Self {
        Self { inner, settings }
    }

    /// Ganho atual (padrão seguro quando as configurações não carregam).
    fn gain(&self) -> f32 {
        clamp_mic_gain(self.settings.load().unwrap_or_default().mic_gain)
    }
}

impl AudioSource for MicrophoneFrontEnd {
    fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        self.inner.list_devices()
    }

    fn start(&self, device_id: Option<String>, consumer: Arc<dyn AudioConsumer>) -> Result<()> {
        let gain = self.gain();
        let (threshold, hangover_ms) = gate_from_env();
        let gate = NoiseGate::new(threshold, hangover_ms);
        // Loga o limiar **efetivo** (o gate já normalizou entradas inválidas).
        tracing::info!(
            gain,
            noise_gate_rms = gate.threshold(),
            noise_gate_hangover_ms = hangover_ms,
            "estágio de entrada do microfone ativo"
        );
        let consumer: Arc<dyn AudioConsumer> = Arc::new(FrontEndConsumer {
            inner: consumer,
            gain,
            gate: Mutex::new(gate),
            scratch: Mutex::new(Vec::new()),
            origin: Instant::now(),
        });
        self.inner.start(device_id, consumer)
    }

    fn stop(&self) -> Result<()> {
        self.inner.stop()
    }

    fn is_running(&self) -> bool {
        self.inner.is_running()
    }
}

/// Consumidor que amplifica, aplica o gate e repassa.
struct FrontEndConsumer {
    inner: Arc<dyn AudioConsumer>,
    gain: f32,
    gate: Mutex<NoiseGate>,
    /// Reaproveitado entre blocos (o callback de áudio é tempo real).
    scratch: Mutex<Vec<f32>>,
    /// Origem do relógio monotônico do gate.
    origin: Instant,
}

impl AudioConsumer for FrontEndConsumer {
    fn consume(&self, samples: &[f32]) {
        let (Ok(mut scratch), Ok(mut gate)) = (self.scratch.lock(), self.gate.lock()) else {
            // Sem os buffers de trabalho, repassa sem tratamento (melhor que
            // perder áudio).
            self.inner.consume(samples);
            return;
        };
        scratch.clear();
        scratch.extend_from_slice(samples);
        apply_gain(&mut scratch, self.gain);
        gate.process(&mut scratch, self.origin.elapsed().as_millis() as u64);
        self.inner.consume(&scratch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::domain::settings::Settings;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Fonte que entrega o consumidor ao teste em vez de capturar de verdade.
    struct CapturingSource {
        last: Mutex<Option<Arc<dyn AudioConsumer>>>,
        started: AtomicUsize,
    }
    impl CapturingSource {
        fn new() -> Self {
            Self {
                last: Mutex::new(None),
                started: AtomicUsize::new(0),
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
            Ok(vec![AudioDevice {
                id: "mic".into(),
                name: "Mic".into(),
                is_default: true,
            }])
        }
        fn start(
            &self,
            _device_id: Option<String>,
            consumer: Arc<dyn AudioConsumer>,
        ) -> Result<()> {
            self.started.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut slot) = self.last.lock() {
                *slot = Some(consumer);
            }
            Ok(())
        }
        fn stop(&self) -> Result<()> {
            Ok(())
        }
        fn is_running(&self) -> bool {
            self.started.load(Ordering::SeqCst) > 0
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

    fn store(gain: f32) -> Arc<InMemorySettingsStore> {
        Arc::new(InMemorySettingsStore::with_settings(Settings {
            mic_gain: gain,
            ..Settings::default()
        }))
    }

    fn front_end(
        inner: &Arc<CapturingSource>,
        gain: f32,
    ) -> (MicrophoneFrontEnd, Arc<RecordingConsumer>) {
        let source = MicrophoneFrontEnd::new(
            Arc::clone(inner) as Arc<dyn AudioSource>,
            store(gain) as Arc<dyn SettingsStore>,
        );
        let sink = Arc::new(RecordingConsumer::new());
        source
            .start(None, Arc::clone(&sink) as Arc<dyn AudioConsumer>)
            .expect("start");
        (source, sink)
    }

    #[test]
    fn amplifies_before_forwarding() {
        let inner = Arc::new(CapturingSource::new());
        let (_source, sink) = front_end(&inner, 8.0);
        inner.consumer().consume(&[0.05, -0.05, 0.5]);
        let got = sink.last();
        assert!((got[0] - 0.4).abs() < 1e-6, "{got:?}");
        assert!((got[1] + 0.4).abs() < 1e-6, "{got:?}");
        assert_eq!(got[2], 1.0, "satura em vez de estourar a faixa");
    }

    #[test]
    fn gain_of_one_forwards_untouched() {
        let inner = Arc::new(CapturingSource::new());
        let (_source, sink) = front_end(&inner, 1.0);
        inner.consumer().consume(&[0.05, 0.5]);
        assert_eq!(sink.last(), vec![0.05, 0.5]);
    }

    #[test]
    fn noise_below_the_threshold_is_silenced() {
        let inner = Arc::new(CapturingSource::new());
        let (_source, sink) = front_end(&inner, 8.0);
        // Ruído baixo: com ganho 8× ainda fica abaixo do limiar de fala.
        inner.consumer().consume(&[0.0002; 1_600]);
        assert!(
            sink.last().iter().all(|sample| *sample == 0.0),
            "ruído devia virar silêncio"
        );
    }

    #[test]
    fn delegates_devices_and_state() {
        let inner = Arc::new(CapturingSource::new());
        let (source, _sink) = front_end(&inner, 8.0);
        assert_eq!(source.list_devices().expect("devices").len(), 1);
        assert!(source.is_running());
        assert!(source.stop().is_ok());
    }

    #[test]
    fn settings_are_read_at_each_start() {
        let inner = Arc::new(CapturingSource::new());
        let settings = store(2.0);
        let source = MicrophoneFrontEnd::new(
            Arc::clone(&inner) as Arc<dyn AudioSource>,
            Arc::clone(&settings) as Arc<dyn SettingsStore>,
        );
        let sink = Arc::new(RecordingConsumer::new());
        source
            .start(None, Arc::clone(&sink) as Arc<dyn AudioConsumer>)
            .expect("start");
        inner.consumer().consume(&[0.25]);
        assert!((sink.last()[0] - 0.5).abs() < 1e-6);

        // O usuário aumentou a sensibilidade: a próxima captura já usa 8×.
        settings
            .save(&Settings {
                mic_gain: 8.0,
                ..Settings::default()
            })
            .expect("save");
        source
            .start(None, Arc::clone(&sink) as Arc<dyn AudioConsumer>)
            .expect("start");
        inner.consumer().consume(&[0.25]);
        assert_eq!(sink.last()[0], 1.0, "satura: 0,25 × 8 > 1");
    }

    #[test]
    fn gate_defaults_align_with_the_speech_threshold_and_can_be_overridden() {
        assert_eq!(resolve_gate(None, None), (SPEECH_RMS_THRESHOLD, 600));
        // Limiar 0 desliga o gate; hangover inválido cai no padrão.
        assert_eq!(resolve_gate(Some("0"), Some("abc")), (0.0, 600));
        assert_eq!(resolve_gate(Some("0.025"), Some("1200")), (0.025, 1_200));
        assert_eq!(resolve_gate(Some("-1"), None), (SPEECH_RMS_THRESHOLD, 600));
    }
}
