//! Comandos IPC de LLM (tradução).

use tauri::State;

use crate::error::Result;
use crate::state::AppState;

/// Traduz um trecho para o idioma de destino configurado.
#[tauri::command(async)]
pub fn llm_translate(state: State<'_, AppState>, text: String) -> Result<String> {
    state.translator.translate(&text)
}
