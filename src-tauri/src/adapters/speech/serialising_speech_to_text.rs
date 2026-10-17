//! Serializa as decodificações de ASR em uma única fila global.
//!
//! Só uma decodificação do Whisper acontece por vez. Isso evita que dois canais
//! (microfone e sistema) disputem CPU ao mesmo tempo, o que degradava a
//! latência e podia estourar o tempo limite. A fila é curta por construção
//! (janelas de poucos segundos), então previsibilidade supera throughput.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::domain::transcription::{SpeechQuality, TranscribedSpan};
use crate::error::{GhostWhisperError, Result};
use crate::ports::speech_to_text::SpeechToText;

/// Tempo máximo de espera na fila antes de desistir (evita travar o pipeline).
const QUEUE_WAIT: Duration = Duration::from_secs(20);
/// Intervalo entre tentativas de aquisição da trava.
const QUEUE_POLL: Duration = Duration::from_millis(5);

/// Adaptador que envolve um ASR com uma trava global compartilhada.
pub struct SerialisingSpeechToText {
    inner: Arc<dyn SpeechToText>,
    gate: Arc<Mutex<()>>,
}

impl SerialisingSpeechToText {
    /// Cria o adaptador; `gate` deve ser compartilhado entre todos os canais.
    pub fn new(inner: Arc<dyn SpeechToText>, gate: Arc<Mutex<()>>) -> Self {
        Self { inner, gate }
    }

    /// Adquire a trava global (com espera limitada).
    fn lock_gate(&self) -> Result<MutexGuard<'_, ()>> {
        let deadline = std::time::Instant::now() + QUEUE_WAIT;
        loop {
            match self.gate.try_lock() {
                Ok(guard) => return Ok(guard),
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    return Err(GhostWhisperError::Other("fila do ASR envenenada".into()));
                }
                Err(std::sync::TryLockError::WouldBlock) => {
                    if std::time::Instant::now() >= deadline {
                        return Err(GhostWhisperError::Other(
                            "fila do ASR ocupada demais (espera >20s)".into(),
                        ));
                    }
                    std::thread::sleep(QUEUE_POLL);
                }
            }
        }
    }
}

impl SpeechToText for SerialisingSpeechToText {
    fn transcribe(&self, samples: &[f32], quality: SpeechQuality) -> Result<Vec<TranscribedSpan>> {
        let _guard = self.lock_gate()?;
        self.inner.transcribe(samples, quality)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::test_support::FakeSpeechToText;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn forwards_quality_in_order() {
        let gate = Arc::new(Mutex::new(()));
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
        let gate = Arc::new(Mutex::new(()));
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
