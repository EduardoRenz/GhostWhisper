//! Portas de entrada da gravação (caso de uso).
//!
//! `RecordingController` é chamado pelo IPC (botão de câmera e modal).
//! `RecordingLifecycle` é chamado pelo `SessionService`: a gravação acompanha o
//! ciclo da sessão (BR-065) sem que a sessão conheça `ffmpeg`.

use crate::domain::recording::{Recording, RecordingState, RecordingTarget};
use crate::error::Result;
use crate::ports::screen_recorder::RecordingCapabilities;
use crate::ports::session_controller::AudioChannel;

/// Controle da gravação da sessão corrente.
///
/// Implementado por `RecordingService`; dobra de teste `FakeRecordingController`.
pub trait RecordingController: Send + Sync {
    /// O ambiente consegue gravar? (habilita/desabilita o botão)
    fn capabilities(&self) -> RecordingCapabilities;

    /// Inicia a gravação na sessão ativa (erro se não houver sessão).
    ///
    /// `target` explícito vem do seletor de janelas; `None` usa a configuração
    /// (janela ativa ou tela inteira).
    fn start(&self, session_id: &str, target: Option<RecordingTarget>) -> Result<()>;

    /// Alvos do seletor: a tela inteira e as janelas (vazio sem X11).
    fn targets(&self) -> Vec<RecordingTarget>;

    /// Pausa a gravação (fecha o segmento); a sessão continua.
    fn pause(&self) -> Result<()>;

    /// Retoma a gravação (abre um novo segmento).
    fn resume(&self) -> Result<()>;

    /// Encerra a gravação e agenda a finalização.
    fn stop(&self) -> Result<()>;

    /// Apaga arquivo e segmentos da sessão; a sessão e a transcrição permanecem.
    fn discard(&self, session_id: &str) -> Result<()>;

    /// Estado atual (para o evento e para a UI).
    fn state(&self) -> RecordingState;

    /// A gravação de uma sessão (histórico/modal).
    fn get_by_session(&self, session_id: &str) -> Result<Option<Recording>>;
}

/// Ganchos do ciclo de vida da sessão (BR-065).
///
/// O `SessionService` chama estes métodos; falhas são registradas em log e
/// **nunca** interrompem a captura/transcrição (RNF-07).
pub trait RecordingLifecycle: Send + Sync {
    /// Sessão aberta. `capturing = false` quando ela nasce pausada (BR-019).
    fn on_session_started(
        &self,
        session_id: &str,
        channels: &[AudioChannel],
        capturing: bool,
    ) -> Result<()>;

    /// A sessão foi pausada: a gravação também pausa.
    fn on_session_paused(&self) -> Result<()>;

    /// A sessão voltou a capturar: retoma a gravação (ou inicia, no auto-start).
    fn on_session_resumed(&self) -> Result<()>;

    /// A sessão foi encerrada: finaliza a gravação.
    fn on_session_stopped(&self) -> Result<()>;
}
