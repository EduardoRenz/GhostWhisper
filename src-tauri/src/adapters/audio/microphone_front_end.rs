//! Estágio de entrada do microfone: **AEC**, **ganho** e **noise gate** antes do
//! pipeline.
//!
//! Transposto do Perssua (renderer do local-whisper: AEC → `micGainNode` →
//! `audioNoiseGate` → PCM16 → `sendAudio`):
//! - **AEC**: subtrai o eco do áudio do sistema (referência) do microfone;
//! - ganho configurável (1×–12×, padrão 8×), aplicado **só no microfone**;
//! - noise gate que **silencia** (não descarta) abaixo do limiar, com *hangover*.
//!
//! Ordem importa: o AEC roda **antes** do ganho (o eco é cancelado no nível
//! original) e o limiar do gate é medido **depois** do ganho, como no Perssua.
//!
//! É um **decorador** sobre a porta [`AudioSource`]: quem o usa continua vendo um
//! `AudioSource` comum, e as configurações são lidas a cada `start()` (mudar o
//! valor vale para a próxima captura).

use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::domain::audio::{
    apply_gain, clamp_mic_gain, AudioDevice, EchoCanceller, NoiseGate, DEFAULT_AEC_DELAY_MS,
    DEFAULT_AEC_TAPS, DEFAULT_NOISE_GATE_HANGOVER_MS, SPEECH_RMS_THRESHOLD, TARGET_SAMPLE_RATE,
};
use crate::error::Result;
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::audio_reference::AudioReference;
use crate::ports::audio_source::AudioSource;
use crate::ports::settings_store::SettingsStore;

/// Variável de ambiente do limiar do gate de ruído (0 desliga).
pub const NOISE_GATE_RMS_ENV: &str = "GHOSTWHISPER_MIC_NOISE_GATE_RMS";
/// Variável de ambiente do hangover do gate (ms).
pub const NOISE_GATE_HANGOVER_ENV: &str = "GHOSTWHISPER_MIC_NOISE_GATE_HANGOVER_MS";
/// Variável de ambiente do atraso da referência do AEC (ms).
pub const AEC_DELAY_ENV: &str = "GHOSTWHISPER_AEC_DELAY_MS";
/// Variável de ambiente do número de coeficientes do AEC.
pub const AEC_TAPS_ENV: &str = "GHOSTWHISPER_AEC_TAPS";

/// Resolve (atraso em amostras, taps) do AEC a partir de overrides de ambiente.
///
/// Regra pura para poder ser testada sem mexer no ambiente do processo.
fn resolve_aec(delay_raw: Option<&str>, taps_raw: Option<&str>) -> (usize, usize) {
    let delay_ms = delay_raw
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .filter(|value| *value >= 0)
        .unwrap_or(DEFAULT_AEC_DELAY_MS)
        .clamp(0, 500);
    let taps = taps_raw
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_AEC_TAPS)
        .clamp(32, 2_048);
    let delay_samples = (TARGET_SAMPLE_RATE as i64 * delay_ms / 1_000) as usize;
    (delay_samples, taps)
}

/// Lê os parâmetros do AEC do ambiente.
fn aec_from_env() -> (usize, usize) {
    let delay = std::env::var(AEC_DELAY_ENV).ok();
    let taps = std::env::var(AEC_TAPS_ENV).ok();
    resolve_aec(delay.as_deref(), taps.as_deref())
}

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

/// Fonte de áudio que aplica AEC, ganho e noise gate de microfone.
pub struct MicrophoneFrontEnd {
    inner: Arc<dyn AudioSource>,
    settings: Arc<dyn SettingsStore>,
    reference: Option<Arc<dyn AudioReference>>,
}

impl MicrophoneFrontEnd {
    /// Envolve uma fonte com o estágio de entrada do microfone.
    pub fn new(inner: Arc<dyn AudioSource>, settings: Arc<dyn SettingsStore>) -> Self {
        Self {
            inner,
            settings,
            reference: None,
        }
    }

    /// Liga o cancelamento de eco usando `reference` (áudio do sistema).
    pub fn with_reference(mut self, reference: Arc<dyn AudioReference>) -> Self {
        self.reference = Some(reference);
        self
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
        let aec_enabled = self
            .settings
            .load()
            .map(|settings| settings.aec_enabled)
            .unwrap_or(true);
        let (delay_samples, taps) = aec_from_env();
        let aec_active = aec_enabled && self.reference.is_some();
        // Loga os parâmetros **efetivos** (o gate já normalizou entradas inválidas).
        tracing::info!(
            gain,
            noise_gate_rms = gate.threshold(),
            noise_gate_hangover_ms = hangover_ms,
            aec = aec_active,
            aec_delay_ms = (delay_samples * 1_000 / TARGET_SAMPLE_RATE as usize) as i64,
            aec_taps = taps,
            "estágio de entrada do microfone ativo"
        );
        let consumer: Arc<dyn AudioConsumer> = Arc::new(FrontEndConsumer {
            inner: consumer,
            gain,
            gate: Mutex::new(gate),
            reference: if aec_active {
                self.reference.clone()
            } else {
                None
            },
            canceller: Mutex::new(EchoCanceller::new(taps)),
            aec_delay_samples: delay_samples,
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

/// Consumidor que cancela o eco, amplifica, aplica o gate e repassa.
struct FrontEndConsumer {
    inner: Arc<dyn AudioConsumer>,
    gain: f32,
    gate: Mutex<NoiseGate>,
    /// Referência do AEC (áudio do sistema); `None` desliga o cancelamento.
    reference: Option<Arc<dyn AudioReference>>,
    canceller: Mutex<EchoCanceller>,
    aec_delay_samples: usize,
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

        // AEC primeiro: o eco é cancelado no nível original (antes do ganho).
        if let Some(reference) = &self.reference {
            let window = reference.delayed(self.aec_delay_samples, scratch.len());
            if let Ok(mut canceller) = self.canceller.lock() {
                canceller.process(&mut scratch, &window);
            }
        }

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

    /// Referência de teste: devolve sempre a janela definida pelo teste.
    struct FakeReference {
        window: Mutex<Vec<f32>>,
    }
    impl AudioReference for FakeReference {
        fn push(&self, _samples: &[f32]) {}
        fn delayed(&self, _delay_samples: usize, len: usize) -> Vec<f32> {
            let guard = self.window.lock().expect("window");
            let mut out = vec![0.0; len];
            let take = guard.len().min(len);
            out[..take].copy_from_slice(&guard[..take]);
            out
        }
    }

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        (samples.iter().map(|value| value * value).sum::<f32>() / samples.len() as f32).sqrt()
    }

    /// Ruído determinístico para o teste do AEC.
    fn noise(len: usize, amplitude: f32, seed: u32) -> Vec<f32> {
        let mut state = seed;
        (0..len)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let unit = (state >> 8) as f32 / (1u32 << 24) as f32 - 0.5;
                unit * 2.0 * amplitude
            })
            .collect()
    }

    #[test]
    fn aec_removes_the_speaker_echo_from_the_microphone() {
        // Ganho 1× para medir só o AEC; sinal alto o bastante para o gate abrir.
        let source = Arc::new(CapturingSource::new());
        let reference = Arc::new(FakeReference {
            window: Mutex::new(Vec::new()),
        });
        let settings = store(1.0);
        let front = MicrophoneFrontEnd::new(
            Arc::clone(&source) as Arc<dyn AudioSource>,
            settings as Arc<dyn SettingsStore>,
        )
        .with_reference(Arc::clone(&reference) as Arc<dyn AudioReference>);
        let sink = Arc::new(RecordingConsumer::new());
        front
            .start(None, Arc::clone(&sink) as Arc<dyn AudioConsumer>)
            .expect("start");
        let consumer = source.consumer();

        let block = 256usize;
        let mut last_input = Vec::new();
        let mut last_output = Vec::new();
        for index in 0..40u32 {
            let reference_block = noise(block, 0.3, index + 1);
            // O microfone ouve 40% do que saiu pelo alto-falante (eco).
            let mic: Vec<f32> = reference_block.iter().map(|value| value * 0.4).collect();
            *reference.window.lock().expect("window") = reference_block;
            consumer.consume(&mic);
            last_input = mic;
            last_output = sink.last();
        }

        assert_eq!(last_output.len(), block);
        let cancelled = rms(&last_output);
        let original = rms(&last_input);
        assert!(
            cancelled < original * 0.5,
            "eco devia cair no estágio de entrada: antes={original} depois={cancelled}"
        );
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
    fn aec_params_have_safe_defaults_and_clamps() {
        let (delay, taps) = resolve_aec(None, None);
        assert_eq!(
            delay,
            (TARGET_SAMPLE_RATE as i64 * DEFAULT_AEC_DELAY_MS / 1_000) as usize
        );
        assert_eq!(taps, DEFAULT_AEC_TAPS);

        // Valores extremos são limitados; lixo cai no padrão.
        assert_eq!(resolve_aec(Some("9999"), Some("99999")).0, 8_000); // 500 ms
        assert_eq!(resolve_aec(Some("-5"), Some("1")).1, 32);
        assert_eq!(resolve_aec(Some("abc"), Some("abc")).1, DEFAULT_AEC_TAPS);
        assert_eq!(resolve_aec(Some("120"), None).0, 1_920);
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
