//! Comandos IPC da gravação da sessão (BR-065…BR-071).

use tauri::State;

use crate::domain::recording::RecordingState;
use crate::error::{GhostWhisperError, Result};
use crate::ports::screen_recorder::RecordingCapabilities;
use crate::state::AppState;

/// O ambiente consegue gravar? (habilita o botão de câmera)
#[tauri::command]
pub fn recording_capabilities(state: State<'_, AppState>) -> RecordingCapabilities {
    state.recording.capabilities()
}

/// Inicia a gravação da sessão ativa.
#[tauri::command(async)]
pub fn recording_start(state: State<'_, AppState>) -> Result<()> {
    let session_id = state.session.current_session_id().ok_or_else(|| {
        GhostWhisperError::Config("inicie uma sessão antes de gravar".to_string())
    })?;
    state.recording.start(&session_id)
}

/// Pausa a gravação (a sessão continua transcrevendo).
#[tauri::command(async)]
pub fn recording_pause(state: State<'_, AppState>) -> Result<()> {
    state.recording.pause()
}

/// Retoma a gravação pausada.
#[tauri::command(async)]
pub fn recording_resume(state: State<'_, AppState>) -> Result<()> {
    state.recording.resume()
}

/// Encerra a gravação e agenda a finalização.
#[tauri::command(async)]
pub fn recording_stop(state: State<'_, AppState>) -> Result<()> {
    state.recording.stop()
}

/// Estado atual (o evento `recording_state` é a fonte principal; isto é o seed).
#[tauri::command]
pub fn recording_state(state: State<'_, AppState>) -> RecordingState {
    state.recording.state()
}

/// Apaga arquivo e segmentos; a sessão e a transcrição permanecem.
#[tauri::command(async)]
pub fn recording_discard(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<()> {
    state.recording.discard(&session_id)
}
