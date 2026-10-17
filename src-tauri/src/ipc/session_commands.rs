//! Comandos IPC de sessão de transcrição.

use serde::Deserialize;
use tauri::State;

use crate::error::Result;
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
#[tauri::command]
pub fn session_start(
    state: State<'_, AppState>,
    channels: Vec<SessionChannelDto>,
) -> Result<String> {
    let channels: Vec<AudioChannel> = channels.into_iter().map(Into::into).collect();
    state.session.start(&channels)
}

/// Encerra a sessão atual.
#[tauri::command]
pub fn session_stop(state: State<'_, AppState>) -> Result<()> {
    state.session.stop()
}

/// Retorna o id da sessão ativa, se houver.
#[tauri::command]
pub fn session_current(state: State<'_, AppState>) -> Result<Option<String>> {
    Ok(state.session.current_session_id())
}
