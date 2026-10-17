//! Porta de fábrica de motores de fala (VAD + ASR) por canal.

use std::sync::Arc;

use crate::error::Result;
use crate::ports::speech_to_text::SpeechToText;
use crate::ports::voice_activity::VoiceActivityDetector;

/// Cria um par VAD + ASR **independente** a cada chamada.
///
/// Cada canal (mic/sistema) recebe seu próprio contexto, evitando serialização
/// e contenção entre pipelines concorrentes.
///
/// Adaptador: `WhisperSpeechEngineFactory`.
pub trait SpeechEngineFactory: Send + Sync {
    /// Cria um motor de fala novo (VAD + ASR próprios).
    fn create(&self) -> Result<(Arc<dyn VoiceActivityDetector>, Arc<dyn SpeechToText>)>;
}
