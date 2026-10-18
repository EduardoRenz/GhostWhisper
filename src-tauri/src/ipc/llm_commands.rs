//! Comandos IPC de LLM (tradução e insights).

use tauri::State;

use crate::error::Result;
use crate::ports::insight_generator::InsightSuggestion;
use crate::state::AppState;

/// Traduz um trecho para o idioma de destino configurado.
#[tauri::command]
pub fn llm_translate(state: State<'_, AppState>, text: String) -> Result<String> {
    state.translator.translate(&text)
}

/// Gera (e persiste) uma sugestão contextual.
///
/// `sessionId` omitido = sessão mais recente. A sugestão volta com o tempo desde
/// o início da sessão, para entrar na linha temporal (task 17).
#[tauri::command]
pub fn llm_insight(
    state: State<'_, AppState>,
    session_id: Option<String>,
) -> Result<InsightSuggestion> {
    state.insight.generate(session_id)
}
