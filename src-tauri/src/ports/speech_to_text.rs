//! Porta de reconhecimento de fala (ASR).

use crate::domain::transcription::{SpeechQuality, TranscribedSpan};
use crate::error::Result;

/// Transcreve um trecho de áudio (mono 16 kHz).
///
/// Adaptador: `WhisperSpeechToText`; dobra de teste: `FakeSpeechToText`.
pub trait SpeechToText: Send + Sync {
    /// Retorna os spans transcritos (tempos relativos ao trecho enviado).
    ///
    /// `quality` define o esforço de decodificação: `Preview` (rápido, revisável)
    /// durante a fala e `Final` (autoritativo, mais caro) ao fechar o turno.
    fn transcribe(&self, samples: &[f32], quality: SpeechQuality) -> Result<Vec<TranscribedSpan>>;
}
