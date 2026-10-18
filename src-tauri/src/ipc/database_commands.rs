//! Comandos IPC de leitura do banco (histórico, linha temporal e busca).

use tauri::State;

use crate::application::session_timeline::{build_timeline, TimelineEntry};
use crate::domain::message::ConversationMessage;
use crate::domain::session::Session;
use crate::domain::transcription::TranscriptSegment;
use crate::error::Result;
use crate::state::AppState;

/// Lista as sessões mais recentes.
#[tauri::command]
pub fn db_list_sessions(state: State<'_, AppState>, limit: Option<usize>) -> Result<Vec<Session>> {
    state.sessions.list_recent(limit.unwrap_or(50))
}

/// Retorna uma sessão pelo id.
#[tauri::command]
pub fn db_get_session(state: State<'_, AppState>, id: String) -> Result<Option<Session>> {
    state.sessions.get(&id)
}

/// Lista as transcrições de uma sessão.
#[tauri::command]
pub fn db_session_transcriptions(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<TranscriptSegment>> {
    state.transcriptions.list_by_session(&session_id)
}

/// Lista as mensagens (sugestões da IA) de uma sessão.
#[tauri::command]
pub fn db_session_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<ConversationMessage>> {
    state.messages.list_by_session(&session_id)
}

/// Linha temporal da sessão: falas + sugestões, já ordenadas no tempo.
#[tauri::command]
pub fn db_session_timeline(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<TimelineEntry>> {
    let segments = state.transcriptions.list_by_session(&session_id)?;
    let messages = state.messages.list_by_session(&session_id)?;
    Ok(build_timeline(&segments, &messages))
}

/// Busca transcrições por texto (FTS5).
#[tauri::command]
pub fn db_search_transcriptions(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<TranscriptSegment>> {
    state.transcriptions.search(&query, limit.unwrap_or(50))
}
