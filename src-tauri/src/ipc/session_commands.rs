//! Comandos IPC de sessão de transcrição.

use serde::Deserialize;
use tauri::State;

use crate::error::Result;
use crate::ipc::session_end_commands::{build_session_end, SessionEndDto};
use crate::ports::session_controller::AudioChannel;
use crate::state::AppState;

/// Canal de áudio escolhido na UI.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionChannelDto {
    Microphone,
    System,
}

impl From<SessionChannelDto> for AudioChannel {
    fn from(value: SessionChannelDto) -> Self {
        match value {
            SessionChannelDto::Microphone => AudioChannel::Microphone,
            SessionChannelDto::System => AudioChannel::System,
        }
    }
}

/// Inicia uma sessão capturando os canais dados; retorna o id da sessão.
#[tauri::command(async)]
pub fn session_start(
    state: State<'_, AppState>,
    channels: Vec<SessionChannelDto>,
) -> Result<String> {
    let channels: Vec<AudioChannel> = channels.into_iter().map(Into::into).collect();
    state.session.start(&channels)
}

/// Encerra a sessão atual e devolve o resumo do modal (BR-068).
#[tauri::command(async)]
pub fn session_stop(state: State<'_, AppState>) -> Result<Option<SessionEndDto>> {
    let session_id = state.session.current_session_id();
    state.session.stop()?;
    match session_id {
        Some(session_id) => build_session_end(&state, &session_id),
        None => Ok(None),
    }
}

/// Renomeia a sessão (título exibido no histórico).
#[tauri::command(async)]
pub fn session_rename(state: State<'_, AppState>, id: String, subject: String) -> Result<()> {
    let subject = subject.trim();
    if subject.is_empty() {
        return Err(crate::error::GhostWhisperError::Config(
            "informe um título para a sessão".to_string(),
        ));
    }
    state.sessions.rename(&id, subject)
}

/// Exclui a sessão e, junto, suas falas e sugestões.
#[tauri::command(async)]
pub fn session_delete(state: State<'_, AppState>, id: String) -> Result<()> {
    // A gravação é apagada junto com a sessão (BR-071/CA-05.4). O descarte
    // vem **antes** do stop: assim a finalização não é agendada para um
    // arquivo que vai deixar de existir.
    state.recording.discard(&id)?;
    if state.session.current_session_id().as_deref() == Some(id.as_str()) {
        state.session.stop()?;
    }
    state.sessions.delete(&id)
}

/// Suspende a captura mantendo a sessão aberta (BR-019).
#[tauri::command(async)]
pub fn session_pause(state: State<'_, AppState>) -> Result<()> {
    state.session.pause()
}

/// Volta a capturar na sessão pausada (BR-019).
#[tauri::command(async)]
pub fn session_resume(state: State<'_, AppState>) -> Result<()> {
    state.session.resume()
}

/// Troca os canais capturados sem encerrar a sessão.
#[tauri::command(async)]
pub fn session_set_channels(
    state: State<'_, AppState>,
    channels: Vec<SessionChannelDto>,
) -> Result<()> {
    let channels: Vec<AudioChannel> = channels.into_iter().map(Into::into).collect();
    state.session.set_channels(&channels)
}

/// Continua uma sessão existente (reativa se encerrada) e captura nela.
#[tauri::command(async)]
pub fn session_continue(
    state: State<'_, AppState>,
    id: String,
    channels: Vec<SessionChannelDto>,
) -> Result<String> {
    let channels: Vec<AudioChannel> = channels.into_iter().map(Into::into).collect();
    state.session.continue_session(&id, &channels)
}

/// Retorna o id da sessão ativa, se houver.
#[tauri::command]
pub fn session_current(state: State<'_, AppState>) -> Result<Option<String>> {
    Ok(state.session.current_session_id())
}

/// Troca o **idioma de origem** da transcrição em tempo real (BR-034).
///
/// Persiste em `source_language` e aplica ao motor em cache no próximo passe;
/// sem sessão ativa apenas persiste (vale na próxima sessão).
#[tauri::command(async)]
pub fn session_set_language(state: State<'_, AppState>, language: String) -> Result<()> {
    let mut settings = state.settings.load()?;
    settings.source_language = language.trim().to_lowercase();
    settings.validate()?;
    state.settings.save(&settings)?;
    state.session.set_language(&settings.source_language)
}
