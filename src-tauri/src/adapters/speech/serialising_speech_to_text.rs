//! Serializa as decodificações de ASR em uma única fila global.
//!
//! Só uma decodificação do Whisper acontece por vez. Isso evita que dois canais
//! (microfone e sistema) disputem CPU ao mesmo tempo, o que degradava a
//! latência e podia estourar o tempo limite. A fila é curta por construção
//! (janelas de poucos segundos), então previsibilidade supera throughput.
//!
//! A fila tem **prioridade**: o passe final (a frase que o usuário acabou de
//! falar) passa na frente de prévias que ainda estão esperando — ver
//! [`SpeechGate`].

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::domain::transcription::{SpeechQuality, TranscribedSpan};
use crate::error::Result;
use crate::ports::speech_to_text::SpeechToText;

use super::speech_gate::{SpeechGate, SpeechGateGuard};

/// Tempo máximo de espera na fila antes de desistir (evita travar o pipeline).
const QUEUE_WAIT: Duration = Duration::from_secs(20);

/// Adaptador que envolve um ASR com uma fila global compartilhada.
pub struct SerialisingSpeechToText {
    inner: Arc<dyn SpeechToText>,
    gate: Arc<SpeechGate>,
}

impl SerialisingSpeechToText {
    /// Cria o adaptador; `gate` deve ser compartilhado entre todos os canais.
    pub fn new(inner: Arc<dyn SpeechToText>, gate: Arc<SpeechGate>) -> Self {
        Self { inner, gate }
    }

    /// Adquire a fila global (com espera limitada e prioridade do final).
    fn lock_gate(&self, quality: SpeechQuality) -> Result<SpeechGateGuard<'_>> {
        self.gate.acquire(quality, QUEUE_WAIT)
    }
}

impl SpeechToText for SerialisingSpeechToText {
    fn transcribe(&self, samples: &[f32], quality: SpeechQuality) -> Result<Vec<TranscribedSpan>> {
        let queued_at = Instant::now();
        let _guard = self.lock_gate(quality)?;
        // Espera na fila **global** (entre canais); o decode vem depois.
        let gate_wait_ms = queued_at.elapsed().as_millis() as i64;
        let started = Instant::now();
        let result = self.inner.transcribe(samples, quality);
        let decode_ms = started.elapsed().as_millis() as i64;
        // Final sempre reporta (ele não deveria esperar); prévia só quando a
        // espera é anormal, para não poluir o log.
        if matches!(quality, SpeechQuality::Final) || gate_wait_ms >= 1_000 {
            tracing::info!(?quality, gate_wait_ms, decode_ms, "fila do ASR");
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::test_support::FakeSpeechToText;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn forwards_quality_in_order() {
        let gate = Arc::new(SpeechGate::new());
        let inner = Arc::new(FakeSpeechToText::new().with_response("ok"));
        let a = SerialisingSpeechToText::new(
            Arc::clone(&inner) as Arc<dyn SpeechToText>,
            Arc::clone(&gate),
        );
        let b = SerialisingSpeechToText::new(
            Arc::clone(&inner) as Arc<dyn SpeechToText>,
            Arc::clone(&gate),
        );

        assert_eq!(
            a.transcribe(&[0.1; 100], SpeechQuality::Final).expect("a")[0].text,
            "ok"
        );
        assert_eq!(
            b.transcribe(&[0.1; 100], SpeechQuality::Preview)
                .expect("b")[0]
                .text,
            "ok"
        );
        assert_eq!(inner.call_count(), 2);
        assert_eq!(
            inner.qualities(),
            vec![SpeechQuality::Final, SpeechQuality::Preview]
        );
    }

    /// ASR que registra quantas decodificações rodam ao mesmo tempo.
    struct SlowAsr {
        concurrent: Arc<AtomicUsize>,
        max_parallel: Arc<AtomicUsize>,
    }

    impl SpeechToText for SlowAsr {
        fn transcribe(
            &self,
            _samples: &[f32],
            _quality: SpeechQuality,
        ) -> Result<Vec<TranscribedSpan>> {
            let now = self.concurrent.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_parallel.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(30));
            self.concurrent.fetch_sub(1, Ordering::SeqCst);
            Ok(vec![TranscribedSpan {
                text: "ok".into(),
                start_ms: 0,
                end_ms: 100,
            }])
        }
    }

    #[test]
    fn never_decodes_in_parallel() {
        let gate = Arc::new(SpeechGate::new());
        let inner = Arc::new(SlowAsr {
            concurrent: Arc::new(AtomicUsize::new(0)),
            max_parallel: Arc::new(AtomicUsize::new(0)),
        });

        let mut handles = Vec::new();
        for _ in 0..4 {
            let asr = SerialisingSpeechToText::new(
                Arc::clone(&inner) as Arc<dyn SpeechToText>,
                Arc::clone(&gate),
            );
            handles.push(std::thread::spawn(move || {
                asr.transcribe(&[0.1; 100], SpeechQuality::Preview)
            }));
        }
        for handle in handles {
            assert!(handle.join().expect("thread").is_ok());
        }

        assert_eq!(
            inner.max_parallel.load(Ordering::SeqCst),
            1,
            "decodificações nunca podem rodar em paralelo"
        );
    }
}
