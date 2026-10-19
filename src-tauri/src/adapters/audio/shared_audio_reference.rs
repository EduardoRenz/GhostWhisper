//! Referência de áudio em memória (anel) e o "tap" que a alimenta.
//!
//! O áudio do sistema é a referência do AEC (BR-029). `SystemAudioReferenceTap` é
//! um **decorador** sobre a fonte do sistema: repassa o áudio ao consumidor de
//! sempre e, de quebra, guarda uma cópia numa janela recente para o microfone.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::domain::audio::{AudioDevice, TARGET_SAMPLE_RATE};
use crate::error::Result;
use crate::ports::audio_consumer::AudioConsumer;
use crate::ports::audio_reference::AudioReference;
use crate::ports::audio_source::AudioSource;

/// Segundos de referência guardados (cobre o atraso configurável com folga).
const REFERENCE_SECONDS: usize = 2;

/// Janela de referência em memória (mono 16 kHz).
pub struct SharedAudioReference {
    samples: Mutex<VecDeque<f32>>,
    capacity: usize,
}

impl SharedAudioReference {
    /// Cria a referência com capacidade padrão (~2 s).
    pub fn new() -> Self {
        Self::with_capacity(TARGET_SAMPLE_RATE as usize * REFERENCE_SECONDS)
    }

    /// Cria a referência com uma capacidade explícita (amostras).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            samples: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity: capacity.max(1),
        }
    }
}

impl Default for SharedAudioReference {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioReference for SharedAudioReference {
    fn push(&self, samples: &[f32]) {
        let Ok(mut guard) = self.samples.lock() else {
            return;
        };
        guard.extend(samples.iter().copied());
        while guard.len() > self.capacity {
            guard.pop_front();
        }
    }

    fn delayed(&self, delay_samples: usize, len: usize) -> Vec<f32> {
        if len == 0 {
            return Vec::new();
        }
        let Ok(guard) = self.samples.lock() else {
            return vec![0.0; len];
        };
        let end = guard.len().saturating_sub(delay_samples);
        let start = end.saturating_sub(len);
        let available = end - start;
        let mut out = Vec::with_capacity(len);
        out.extend(std::iter::repeat(0.0).take(len - available));
        out.extend(guard.iter().skip(start).take(available).copied());
        out
    }
}

/// Consumidor que guarda uma cópia na referência e repassa ao consumidor real.
struct TapConsumer {
    inner: Arc<dyn AudioConsumer>,
    reference: Arc<dyn AudioReference>,
}

impl AudioConsumer for TapConsumer {
    fn consume(&self, samples: &[f32]) {
        if !samples.is_empty() {
            self.reference.push(samples);
        }
        self.inner.consume(samples);
    }
}

/// Fonte de áudio do sistema que alimenta a referência do AEC.
pub struct SystemAudioReferenceTap {
    inner: Arc<dyn AudioSource>,
    reference: Arc<dyn AudioReference>,
}

impl SystemAudioReferenceTap {
    /// Envolve a fonte do sistema com o tap da referência.
    pub fn new(inner: Arc<dyn AudioSource>, reference: Arc<dyn AudioReference>) -> Self {
        Self { inner, reference }
    }
}

impl AudioSource for SystemAudioReferenceTap {
    fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        self.inner.list_devices()
    }

    fn start(&self, device_id: Option<String>, consumer: Arc<dyn AudioConsumer>) -> Result<()> {
        let tapped: Arc<dyn AudioConsumer> = Arc::new(TapConsumer {
            inner: consumer,
            reference: Arc::clone(&self.reference),
        });
        self.inner.start(device_id, tapped)
    }

    fn stop(&self) -> Result<()> {
        self.inner.stop()
    }

    fn is_running(&self) -> bool {
        self.inner.is_running()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: usize = TARGET_SAMPLE_RATE as usize;

    #[test]
    fn delayed_returns_the_recent_window() {
        let reference = SharedAudioReference::with_capacity(RATE);
        reference.push(&[1.0, 2.0, 3.0, 4.0, 5.0]);

        // Sem atraso: últimos 3.
        assert_eq!(reference.delayed(0, 3), vec![3.0, 4.0, 5.0]);
        // Atraso de 2: janela termina em 3 ⇒ [1, 2, 3].
        assert_eq!(reference.delayed(2, 3), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn delayed_pads_with_zeros_when_history_is_short() {
        let reference = SharedAudioReference::with_capacity(RATE);
        reference.push(&[1.0, 2.0]);
        assert_eq!(reference.delayed(0, 4), vec![0.0, 0.0, 1.0, 2.0]);
        assert_eq!(reference.delayed(10, 2), vec![0.0, 0.0]);
    }

    #[test]
    fn ring_buffer_keeps_only_the_capacity() {
        let reference = SharedAudioReference::with_capacity(3);
        reference.push(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(reference.delayed(0, 3), vec![2.0, 3.0, 4.0]);
    }

    /// Consumidor que registra o que recebe.
    struct RecordingConsumer {
        received: Mutex<Vec<f32>>,
    }
    impl AudioConsumer for RecordingConsumer {
        fn consume(&self, samples: &[f32]) {
            self.received
                .lock()
                .expect("received")
                .extend_from_slice(samples);
        }
    }

    /// Fonte que entrega o consumidor ao teste.
    struct CapturingSource {
        last: Mutex<Option<Arc<dyn AudioConsumer>>>,
    }
    impl AudioSource for CapturingSource {
        fn list_devices(&self) -> Result<Vec<AudioDevice>> {
            Ok(vec![])
        }
        fn start(&self, _device: Option<String>, consumer: Arc<dyn AudioConsumer>) -> Result<()> {
            *self.last.lock().expect("last") = Some(consumer);
            Ok(())
        }
        fn stop(&self) -> Result<()> {
            Ok(())
        }
        fn is_running(&self) -> bool {
            false
        }
    }

    #[test]
    fn tap_forwards_to_the_consumer_and_fills_the_reference() {
        let inner = Arc::new(CapturingSource {
            last: Mutex::new(None),
        });
        let reference: Arc<dyn AudioReference> = Arc::new(SharedAudioReference::new());
        let tap = SystemAudioReferenceTap::new(
            Arc::clone(&inner) as Arc<dyn AudioSource>,
            Arc::clone(&reference),
        );
        let sink = Arc::new(RecordingConsumer {
            received: Mutex::new(Vec::new()),
        });
        tap.start(None, Arc::clone(&sink) as Arc<dyn AudioConsumer>)
            .expect("start");

        inner
            .last
            .lock()
            .expect("last")
            .as_ref()
            .expect("consumer")
            .consume(&[0.5, -0.5]);

        assert_eq!(*sink.received.lock().expect("received"), vec![0.5, -0.5]);
        assert_eq!(reference.delayed(0, 2), vec![0.5, -0.5]);
    }
}
