//! Comandos IPC da gravação da sessão (BR-065…BR-071).

use tauri::{Emitter, Manager, State};

use crate::domain::recording::{RecordingState, RecordingTarget};
use crate::error::{GhostWhisperError, Result};
use crate::ports::screen_recorder::RecordingCapabilities;
use crate::state::AppState;

/// O ambiente consegue gravar? (habilita o botão de câmera)
#[tauri::command]
pub fn recording_capabilities(state: State<'_, AppState>) -> RecordingCapabilities {
    state.recording.capabilities()
}

/// Inicia a gravação da sessão ativa.
///
/// `target` explícito vem do seletor de janelas; omitido, usa a configuração.
#[tauri::command(async)]
pub fn recording_start(
    state: State<'_, AppState>,
    target: Option<RecordingTarget>,
) -> Result<()> {
    let session_id = state.session.current_session_id().ok_or_else(|| {
        GhostWhisperError::Config("inicie uma sessão antes de gravar".to_string())
    })?;
    state.recording.start(&session_id, target)
}

/// Alvos do seletor: tela inteira + janelas (vazio sem X11).
#[tauri::command]
pub fn recording_targets(state: State<'_, AppState>) -> Vec<RecordingTarget> {
    state.recording.targets()
}

/// Mostra o seletor de janelas.
#[tauri::command(async)]
pub fn recording_picker_open(app: tauri::AppHandle, _state: State<'_, AppState>) -> Result<()> {
    if let Some(window) = app.get_webview_window("recording-picker") {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        // A janela existe desde o boot: avisa para reler a lista de alvos.
        let _ = window.emit("recording_picker_open", ());
    }
    Ok(())
}

/// Esconde o seletor de janelas.
#[tauri::command(async)]
pub fn recording_picker_close(app: tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("recording-picker") {
        let _ = window.hide();
    }
    Ok(())
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
