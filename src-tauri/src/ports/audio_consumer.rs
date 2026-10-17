//! Porta de consumo de amostras de áudio.

/// Recebe blocos de PCM **f32 mono a 16 kHz** já normalizados.
///
/// Adaptadores: `LevelMeter` (medidor), `TranscriptionConsumer` (futuro, Tarefa 04).
pub trait AudioConsumer: Send + Sync {
    /// Consome um bloco de amostras normalizadas em `[-1.0, 1.0]`.
    fn consume(&self, samples: &[f32]);
}
