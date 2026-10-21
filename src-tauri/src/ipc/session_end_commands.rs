//! Comandos IPC do modal "Sessão Finalizada" (BR-068).
//!
//! O `session_stop` devolve o resumo; o overlay abre a janela `session-end` e
//! ela busca o contexto ao montar (padrão do `capture_pending`).

use serde::Serialize;
use tauri::{Emitter, Manager, State};

use crate::domain::recording::Recording;
use crate::error::Result;
use crate::ipc::window_visibility::show_above;
use crate::state::AppState;

/// Evento avisando a janela `session-end` de que há um novo contexto.
pub const SESSION_END_OPEN_EVENT: &str = "session_end_open";

/// Resumo de mídia exibido no modal.
#[derive(Debug, Clone, Serialize)]
pub struct RecordingSummaryDto {
    pub kind: String,
    pub status: String,
    pub path: Option<String>,
    /// Pasta dos segmentos (usada quando a finalização falhou).
    pub segments_dir: Option<String>,
    pub size_bytes: u64,
    pub duration_ms: i64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub error: Option<String>,
}

impl From<Recording> for RecordingSummaryDto {
    fn from(recording: Recording) -> Self {
        Self {
            kind: recording.kind.as_str().to_string(),
            status: recording.status.as_str().to_string(),
            path: recording.path,
            segments_dir: recording.segments_dir,
            size_bytes: recording.size_bytes,
            duration_ms: recording.duration_ms,
            width: recording.width,
            height: recording.height,
            fps: recording.fps,
            error: recording.error,
        }
    }
}

/// Payload do modal.
#[derive(Debug, Clone, Serialize)]
pub struct SessionEndDto {
    pub session_id: String,
    pub subject: Option<String>,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub duration_ms: i64,
    pub transcriptions: u32,
    pub suggestions: u32,
    pub recording: Option<RecordingSummaryDto>,
}

/// Monta o resumo de uma sessão encerrada.
pub fn build_session_end(state: &AppState, session_id: &str) -> Result<Option<SessionEndDto>> {
    let Some(session) = state.sessions.get(session_id)? else {
        return Ok(None);
    };
    let transcriptions = state.transcriptions.list_by_session(session_id)?.len() as u32;
    let suggestions = state.messages.list_by_session(session_id)?.len() as u32;
    let ended_at_ms = session.ended_at_ms.unwrap_or_else(|| state.clock.now_ms());
    let recording = state
        .recording
        .get_by_session(session_id)?
        .map(RecordingSummaryDto::from);

    Ok(Some(SessionEndDto {
        session_id: session.id,
        subject: session.subject,
        started_at_ms: session.started_at_ms,
        ended_at_ms,
        duration_ms: (ended_at_ms - session.started_at_ms).max(0),
        transcriptions,
        suggestions,
        recording,
    }))
}

/// Marca a sessão do modal e mostra a janela.
#[tauri::command(async)]
pub fn session_end_open(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<()> {
    state.pending_session_end.set(&session_id)?;
    if let Some(window) = app.get_webview_window("session-end") {
        let _ = window.center();
        let _ = show_above(&window);
        let _ = window.set_focus();
        // A janela existe desde o boot (oculta): avisa para reler o contexto.
        let _ = window.emit(SESSION_END_OPEN_EVENT, ());
    }
    Ok(())
}

/// Contexto do modal (recarregável enquanto ele estiver aberto).
#[tauri::command(async)]
pub fn session_end_context(state: State<'_, AppState>) -> Result<Option<SessionEndDto>> {
    let Some(session_id) = state.pending_session_end.get()? else {
        return Ok(None);
    };
    build_session_end(&state, &session_id)
}

/// Fecha o modal e limpa o contexto.
#[tauri::command(async)]
pub fn session_end_close(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    state.pending_session_end.clear()?;
    if let Some(window) = app.get_webview_window("session-end") {
        let _ = window.hide();
    }
    Ok(())
}
