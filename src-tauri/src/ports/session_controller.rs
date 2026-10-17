//! Porta de controle de sessão (inbound port / caso de uso).

use crate::error::Result;

/// Canal de áudio de uma sessão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChannel {
    /// Microfone do usuário (`ME`).
    Microphone,
    /// Áudio do sistema (`THEY`).
    System,
}

impl AudioChannel {
    /// Valor persistido/exposto em eventos.
    pub fn as_str(&self) -> &'static str {
        match self {
            AudioChannel::Microphone => "microphone",
            AudioChannel::System => "system",
        }
    }
}

/// Inicia/encerra sessões de transcrição.
///
/// Implementado pela aplicação (`SessionService`) e chamado pelo IPC.
pub trait SessionController: Send + Sync {
    /// Inicia uma sessão capturando **todos** os canais dados; retorna o id.
    fn start(&self, channels: &[AudioChannel]) -> Result<String>;
    /// Encerra a sessão atual (se houver).
    fn stop(&self) -> Result<()>;
    /// Id da sessão ativa, se houver.
    fn current_session_id(&self) -> Option<String>;
}
