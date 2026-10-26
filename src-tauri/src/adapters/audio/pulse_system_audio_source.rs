//! Adaptador de captura do áudio do sistema.
//!
//! Lista monitores de saída via `pactl` e grava do monitor selecionado com a API
//! simple do PulseAudio (compatível com PipeWire). O servidor faz o remix para
//! mono 16 kHz, então o cliente só normaliza as amostras.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;

use crate::domain::audio::{AudioDevice, TARGET_SAMPLE_RATE};
use crate::error::{GhostWhisperError, Result};
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::audio_source::AudioSource;

use super::pactl;

/// Captura o monitor de saída do sistema (PulseAudio/PipeWire).
pub struct PulseSystemAudioSource {
    stop: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Default for PulseSystemAudioSource {
    fn default() -> Self {
        Self::new()
    }
}

impl PulseSystemAudioSource {
    /// Cria a fonte (nenhuma captura é iniciada aqui).
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
        }
    }
}

impl AudioSource for PulseSystemAudioSource {
    fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        let sources = pactl::run_pactl(&["list", "short", "sources"])?;
        let default_monitor = pactl::run_pactl(&["get-default-sink"])
            .ok()
            .map(|sink| format!("{}.monitor", sink.trim()))
            .filter(|name| !name.is_empty());
        Ok(parse_monitors(&sources, default_monitor.as_deref()))
    }

    fn start(&self, device_id: Option<String>, consumer: Arc<dyn AudioConsumer>) -> Result<()> {
        self.stop()?;

        let source_name = match device_id {
            Some(name) => name,
            None => default_monitor_name()?,
        };

        self.stop.store(false, Ordering::Relaxed);
        self.running.store(true, Ordering::Relaxed);

        let stop = Arc::clone(&self.stop);
        let running = Arc::clone(&self.running);
        let handle = thread::spawn(move || {
            if let Err(error) = run_capture(&source_name, consumer, Arc::clone(&stop)) {
                tracing::error!("captura do sistema falhou: {error}");
            }
            running.store(false, Ordering::Relaxed);
        });

        if let Ok(mut guard) = self.thread.lock() {
            *guard = Some(handle);
        }
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut guard) = self.thread.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

/// Nome do monitor do sink padrão (`<sink>.monitor`).
pub(crate) fn default_monitor_name() -> Result<String> {
    let sink = pactl::run_pactl(&["get-default-sink"])?;
    let sink = sink.trim();
    if sink.is_empty() {
        return Err(GhostWhisperError::Other(
            "sink padrão indisponível".to_string(),
        ));
    }
    Ok(format!("{sink}.monitor"))
}

/// Grava do `source_name` normalizando para f32 mono 16 kHz.
fn run_capture(
    source_name: &str,
    consumer: Arc<dyn AudioConsumer>,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    let spec = Spec {
        format: Format::S16le,
        channels: 1,
        rate: TARGET_SAMPLE_RATE,
    };

    let simple = Simple::new(
        None,
        "GhostWhisper",
        Direction::Record,
        Some(source_name),
        "system-capture",
        &spec,
        None,
        None,
    )
    .map_err(|e| GhostWhisperError::Other(format!("falha ao abrir captura do sistema: {e}")))?;

    let mut buffer = vec![0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        match simple.read(&mut buffer) {
            Ok(()) => {
                let samples: Vec<f32> = buffer
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0)
                    .collect();
                if !samples.is_empty() {
                    consumer.consume(&samples);
                }
            }
            Err(error) => {
                tracing::error!("erro na leitura do áudio do sistema: {error}");
                break;
            }
        }
    }

    Ok(())
}

/// Extrai monitores de saída da saída de `pactl list short sources`.
///
/// Linha: `<índice>\t<nome>\t<driver>\t<spec>\t<estado>`.
pub fn parse_monitors(output: &str, default_monitor: Option<&str>) -> Vec<AudioDevice> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let _index = fields.next()?;
            let name = fields.next()?.trim();
            if name.is_empty() || !name.ends_with(".monitor") {
                return None;
            }
            Some(AudioDevice {
                id: name.to_string(),
                name: name.to_string(),
                is_default: default_monitor == Some(name),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str =
        "49\talsa_output.usb-Headset.monitor\tPipeWire\ts16le 2ch 48000Hz\tSUSPENDED\n\
50\talsa_input.usb-Headset.mono-fallback\tPipeWire\ts16le 1ch 48000Hz\tSUSPENDED\n\
51\talsa_output.pci.hdmi.monitor\tPipeWire\ts32le 2ch 48000Hz\tSUSPENDED";

    #[test]
    fn lists_only_monitors() {
        let devices = parse_monitors(SAMPLE, None);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].id, "alsa_output.usb-Headset.monitor");
        assert_eq!(devices[1].id, "alsa_output.pci.hdmi.monitor");
    }

    #[test]
    fn marks_default_monitor() {
        let devices = parse_monitors(SAMPLE, Some("alsa_output.pci.hdmi.monitor"));
        let default = devices.iter().find(|d| d.is_default).expect("default");
        assert_eq!(default.id, "alsa_output.pci.hdmi.monitor");
    }

    #[test]
    fn source_starts_not_running() {
        assert!(!PulseSystemAudioSource::new().is_running());
    }

    // Testes de integração com o servidor real (rode com `--ignored`).

    #[test]
    #[ignore = "requer servidor Pulse/PipeWire em execução"]
    fn lists_monitors_on_real_server() {
        let devices = PulseSystemAudioSource::new().list_devices().expect("lista");
        assert!(!devices.is_empty(), "esperado ao menos um monitor");
        assert!(
            devices.iter().any(|d| d.is_default),
            "esperado monitor padrão"
        );
    }

    #[test]
    #[ignore = "requer servidor Pulse/PipeWire e reprodução de áudio"]
    fn captures_default_monitor_when_audio_plays() {
        use std::io::Write;
        use std::sync::atomic::AtomicUsize;

        struct Counter(AtomicUsize);
        impl AudioConsumer for Counter {
            fn consume(&self, samples: &[f32]) {
                self.0.fetch_add(samples.len(), Ordering::Relaxed);
            }
        }

        let source = PulseSystemAudioSource::new();
        let counter = Arc::new(Counter(AtomicUsize::new(0)));
        source.start(None, counter.clone()).expect("start");
        std::thread::sleep(std::time::Duration::from_millis(400));

        // Toca um tom no sink padrão para gerar dados no monitor.
        let path = std::env::temp_dir().join("ghostwhisper_tone.wav");
        let rate = 48_000u32;
        let samples = rate; // 1 s
        let data_len = samples * 2;
        {
            let mut file = std::fs::File::create(&path).expect("wav");
            file.write_all(b"RIFF").unwrap();
            file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
            file.write_all(b"WAVEfmt ").unwrap();
            file.write_all(&16u32.to_le_bytes()).unwrap();
            file.write_all(&1u16.to_le_bytes()).unwrap();
            file.write_all(&1u16.to_le_bytes()).unwrap();
            file.write_all(&rate.to_le_bytes()).unwrap();
            file.write_all(&(rate * 2).to_le_bytes()).unwrap();
            file.write_all(&2u16.to_le_bytes()).unwrap();
            file.write_all(&16u16.to_le_bytes()).unwrap();
            file.write_all(b"data").unwrap();
            file.write_all(&data_len.to_le_bytes()).unwrap();
            for i in 0..samples {
                let t = i as f32 / rate as f32;
                let v = (t * 440.0 * std::f32::consts::PI * 2.0).sin() * 0.3;
                file.write_all(&((v * 32767.0) as i16).to_le_bytes())
                    .unwrap();
            }
        }
        let _ = std::process::Command::new("paplay").arg(&path).status();
        std::thread::sleep(std::time::Duration::from_millis(300));
        source.stop().expect("stop");
        let _ = std::fs::remove_file(&path);

        let captured = counter.0.load(Ordering::Relaxed);
        assert!(captured > 0, "nenhuma amostra capturada do monitor");
    }
}
