//! Porta de armazenamento/download de modelos (VAD e ASR).

use std::path::PathBuf;

use crate::error::Result;

/// Resolve o caminho local de um modelo, baixando-o se necessário.
///
/// Adaptador: `FileModelStore`.
pub trait ModelStore: Send + Sync {
    /// Caminho do modelo Silero VAD (baixa/verifica se preciso).
    fn vad_model_path(&self) -> Result<PathBuf>;
    /// Caminho de um modelo whisper.cpp pelo nome (`tiny`, `base`, ...).
    fn whisper_model_path(&self, name: &str) -> Result<PathBuf>;
}
