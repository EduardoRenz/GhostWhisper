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

/// Inicia/encerra/pausa sessões de transcrição.
///
/// Implementado pela aplicação (`SessionService`) e chamado pelo IPC.
///
/// Conceito de sessão (BR-019): **pausar** suspende a captura mas mantém a
/// sessão aberta (mesmo id); **retomar** volta a capturar nela; **continuar**
/// reativa uma sessão encerrada; só [`start`](SessionController::start) começa um
/// id novo.
pub trait SessionController: Send + Sync {
    /// Inicia uma **nova** sessão capturando todos os canais dados; retorna o id.
    fn start(&self, channels: &[AudioChannel]) -> Result<String>;
    /// Encerra a sessão atual (se houver).
    fn stop(&self) -> Result<()>;
    /// Suspende a captura mantendo a sessão aberta.
    fn pause(&self) -> Result<()>;
    /// Volta a capturar na sessão pausada.
    fn resume(&self) -> Result<()>;
    /// Troca os canais capturados **mantendo a sessão aberta** (inclusive
    /// pausada: a escolha vale para a próxima captura).
    fn set_channels(&self, channels: &[AudioChannel]) -> Result<()>;
    /// Continua uma sessão existente (reativa se encerrada) e captura nela.
    fn continue_session(&self, id: &str, channels: &[AudioChannel]) -> Result<String>;
    /// Troca o **idioma de origem** da transcrição na sessão em andamento
    /// (BR-034): aplica ao próximo passe, sem reiniciar nem recarregar o modelo.
    fn set_language(&self, language: &str) -> Result<()>;
    /// Id da sessão ativa (inclui a pausada), se houver.
    fn current_session_id(&self) -> Option<String>;
}
