//! Comandos IPC de LLM (tradução e insights).

use tauri::State;

use crate::error::Result;
use crate::state::AppState;

/// Traduz um trecho para o idioma de destino configurado.
#[tauri::command]
pub fn llm_translate(state: State<'_, AppState>, text: String) -> Result<String> {
    state.translator.translate(&text)
}

/// Gera uma sugestão contextual (`session_id` omitido = sessão mais recente).
#[tauri::command]
pub fn llm_insight(state: State<'_, AppState>, session_id: Option<String>) -> Result<String> {
    state.insight.generate(session_id)
}
