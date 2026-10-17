//! Porta de detecção de atividade de voz (VAD).

use crate::domain::transcription::SpeechSpan;
use crate::error::Result;

/// Segmenta fala dentro de um buffer de áudio (mono 16 kHz).
///
/// Adaptador: `SileroVoiceActivityDetector`; dobra de teste: `FakeVoiceActivityDetector`.
pub trait VoiceActivityDetector: Send + Sync {
    /// Retorna os trechos com fala (tempos relativos ao início do buffer).
    fn speech_segments(&self, samples: &[f32]) -> Result<Vec<SpeechSpan>>;
}
