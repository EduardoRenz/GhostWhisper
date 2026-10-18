//! Porta de fábrica de motores de fala (ASR) por canal.

use std::sync::Arc;

use crate::error::Result;
use crate::ports::speech_to_text::SpeechToText;

/// Fornece o motor de ASR usado por um canal (mic/sistema).
///
/// A implementação pode **compartilhar** um único contexto entre os canais
/// (Perssua mantém um `whisper-server` só), então as chamadas de `create()`
/// podem devolver a mesma instância — a serialização de decodificação é feita
/// por `SerialisingSpeechToText`.
///
/// Adaptador: `WhisperSpeechEngineFactory`.
pub trait SpeechEngineFactory: Send + Sync {
    /// Cria (ou reusa) o motor de fala do canal.
    fn create(&self) -> Result<Arc<dyn SpeechToText>>;
}
