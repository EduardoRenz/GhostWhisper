//! Adaptador de captura de microfone sobre `cpal`.
//!
//! O stream do `cpal` é criado e mantido dentro de uma thread dedicada; a porta
//! apenas sinaliza início/parada, evitando exigências de `Send`/`Sync` do stream.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};

use crate::domain::audio::AudioDevice;
use crate::error::{GhostWhisperError, Result};
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::audio_source::AudioSource;

use super::pcm_resampler;

/// Captura de microfone via `cpal` (ALSA/PulseAudio/PipeWire).
pub struct CpalMicrophoneSource {
    stop: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Default for CpalMicrophoneSource {
    fn default() -> Self {
        Self::new()
    }
}

impl CpalMicrophoneSource {
    /// Cria a fonte de microfone (nenhuma captura é iniciada aqui).
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
        }
    }
}

impl AudioSource for CpalMicrophoneSource {
    fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        let host = cpal::default_host();
        let default_name = host.default_input_device().and_then(|d| d.name().ok());

        let devices = host
            .input_devices()
            .map_err(|e| GhostWhisperError::Other(format!("falha ao listar dispositivos: {e}")))?
            .filter_map(|device| {
                let name = device.name().ok()?;
                Some(AudioDevice {
                    is_default: default_name.as_deref() == Some(name.as_str()),
                    id: name.clone(),
                    name,
                })
            })
            .collect();

        Ok(devices)
    }

    fn start(&self, device_id: Option<String>, consumer: Arc<dyn AudioConsumer>) -> Result<()> {
        self.stop()?;

        self.stop.store(false, Ordering::Relaxed);
        self.running.store(true, Ordering::Relaxed);

        let stop = Arc::clone(&self.stop);
        let running = Arc::clone(&self.running);
        let handle = thread::spawn(move || {
            if let Err(error) = run_capture(device_id, consumer, Arc::clone(&stop)) {
                tracing::error!("captura de microfone falhou: {error}");
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

/// Abre o dispositivo, toca o stream e aguarda o sinal de parada.
fn run_capture(
    device_id: Option<String>,
    consumer: Arc<dyn AudioConsumer>,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    let host = cpal::default_host();
    let device = match device_id {
        Some(id) => host
            .input_devices()
            .map_err(|e| GhostWhisperError::Other(format!("falha ao listar dispositivos: {e}")))?
            .find(|d| d.name().map(|n| n == id).unwrap_or(false))
            .ok_or_else(|| GhostWhisperError::Other(format!("dispositivo não encontrado: {id}")))?,
        None => host
            .default_input_device()
            .ok_or_else(|| GhostWhisperError::Other("nenhum microfone de entrada".to_string()))?,
    };

    let supported = device.default_input_config().map_err(|e| {
        GhostWhisperError::Other(format!("configuração de entrada indisponível: {e}"))
    })?;

    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.config();
    let channels = config.channels;
    let rate = config.sample_rate.0;

    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(&device, &config, channels, rate, consumer),
        SampleFormat::I16 => build_stream::<i16>(&device, &config, channels, rate, consumer),
        SampleFormat::U16 => build_stream::<u16>(&device, &config, channels, rate, consumer),
        other => Err(GhostWhisperError::Other(format!(
            "formato de amostra não suportado: {other:?}"
        ))),
    }?;

    stream
        .play()
        .map_err(|e| GhostWhisperError::Other(format!("falha ao iniciar o stream: {e}")))?;

    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(50));
    }

    drop(stream);
    Ok(())
}

/// Constrói o stream de entrada para um formato de amostra `T`.
fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: u16,
    rate: u32,
    consumer: Arc<dyn AudioConsumer>,
) -> Result<cpal::Stream>
where
    T: SizedSample + Sample,
    f32: FromSample<T>,
{
    let error_consumer = Arc::clone(&consumer);
    let stream = device
        .build_input_stream(
            config,
            move |data: &[T], _info: &cpal::InputCallbackInfo| {
                let interleaved: Vec<f32> = data.iter().map(|&s| f32::from_sample(s)).collect();
                let mono_16k = pcm_resampler::to_target_mono(&interleaved, channels, rate);
                if !mono_16k.is_empty() {
                    consumer.consume(&mono_16k);
                }
            },
            move |error| {
                tracing::error!("erro no stream de áudio: {error}");
                let _ = &error_consumer;
            },
            None,
        )
        .map_err(|e| GhostWhisperError::Other(format!("falha ao abrir o stream: {e}")))?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_starts_not_running() {
        let source = CpalMicrophoneSource::new();
        assert!(!source.is_running());
    }
}
