//! Detecção de voz (VAD) com o modelo Silero via whisper.cpp.

use std::sync::{Arc, Mutex};

use whisper_rs::{WhisperVadContext, WhisperVadContextParams, WhisperVadParams};

use crate::domain::transcription::SpeechSpan;
use crate::error::{GhostWhisperError, Result};
use crate::ports::model_store::ModelStore;
use crate::ports::voice_activity::VoiceActivityDetector;

/// Timestamps do VAD vêm em centissegundos (10 ms).
const CENTISECONDS_TO_MS: f32 = 10.0;

/// VAD Silero carregado sob demanda a partir do `ModelStore`.
pub struct SileroVoiceActivityDetector {
    models: Arc<dyn ModelStore>,
    context: Mutex<Option<WhisperVadContext>>,
}

impl SileroVoiceActivityDetector {
    /// Cria o detector (o modelo é carregado na primeira detecção).
    pub fn new(models: Arc<dyn ModelStore>) -> Self {
        Self {
            models,
            context: Mutex::new(None),
        }
    }

    fn with_context<T>(&self, f: impl FnOnce(&mut WhisperVadContext) -> Result<T>) -> Result<T> {
        let mut guard = self
            .context
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock do VAD envenenado".to_string()))?;

        if guard.is_none() {
            let path = self.models.vad_model_path()?;
            let path_str = path.to_string_lossy().to_string();
            let mut params = WhisperVadContextParams::new();
            params.set_use_gpu(false);
            let context = WhisperVadContext::new(&path_str, params)
                .map_err(|e| GhostWhisperError::Other(format!("falha ao carregar VAD: {e}")))?;
            *guard = Some(context);
        }

        f(guard.as_mut().expect("contexto inicializado"))
    }
}

impl VoiceActivityDetector for SileroVoiceActivityDetector {
    fn speech_segments(&self, samples: &[f32]) -> Result<Vec<SpeechSpan>> {
        if samples.is_empty() {
            return Ok(Vec::new());
        }

        self.with_context(|context| {
            let params = WhisperVadParams::new();
            let segments = context
                .segments_from_samples(params, samples)
                .map_err(|e| GhostWhisperError::Other(format!("falha no VAD: {e}")))?;

            let spans = segments
                .map(|segment| SpeechSpan {
                    start_ms: (segment.start * CENTISECONDS_TO_MS).round() as i64,
                    end_ms: (segment.end * CENTISECONDS_TO_MS).round() as i64,
                })
                .collect();

            Ok(spans)
        })
    }
}
