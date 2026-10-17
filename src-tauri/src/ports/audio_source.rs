//! Porta de captura de áudio (microfone ou áudio do sistema).

use std::sync::Arc;

use crate::domain::audio::AudioDevice;
use crate::error::Result;
use crate::ports::audio_consumer::AudioConsumer;

/// Fonte de áudio capturável.
///
/// Adaptadores: `CpalMicrophoneSource` (mic), `PulseSystemAudioSource` (sistema).
pub trait AudioSource: Send + Sync {
    /// Lista os dispositivos de entrada disponíveis.
    fn list_devices(&self) -> Result<Vec<AudioDevice>>;
    /// Inicia a captura do dispositivo (`None` = padrão) entregando PCM ao `consumer`.
    fn start(&self, device_id: Option<String>, consumer: Arc<dyn AudioConsumer>) -> Result<()>;
    /// Interrompe a captura em andamento.
    fn stop(&self) -> Result<()>;
    /// `true` se há captura ativa.
    fn is_running(&self) -> bool;
}
