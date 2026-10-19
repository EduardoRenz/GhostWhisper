//! Comandos IPC de sugestões e agentes.

use tauri::State;

use crate::domain::assistant::AssistantMode;
use crate::error::Result;
use crate::ports::suggestion_generator::{PinSuggestionRequest, Suggestion, SuggestionRequest};
use crate::state::AppState;

/// Lista os agentes embutidos + personalizados (com os prompts, para duplicar).
#[tauri::command(async)]
pub fn assistant_modes_list(state: State<'_, AppState>) -> Result<Vec<AssistantMode>> {
    let settings = state.settings.load()?;
    let mut modes = crate::application::prompts::assistant::builtin_modes();
    modes.extend(settings.custom_assistant_modes);
    Ok(modes)
}

/// Gera uma sugestão **efêmera** para a janela (não grava nada).
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub fn suggestion_generate(
    state: State<'_, AppState>,
    session_id: Option<String>,
    mode_id: Option<String>,
    since_ms: Option<i64>,
    last_suggestion: Option<String>,
    extra_context: Option<String>,
    fast_first: Option<bool>,
    force: Option<bool>,
) -> Result<Suggestion> {
    state.suggestions.generate(
        SuggestionRequest::ephemeral(session_id, mode_id.unwrap_or_default())
            .with_since_ms(since_ms)
            .with_last_suggestion(last_suggestion)
            .with_extra_context(extra_context)
            .with_fast_first(fast_first.unwrap_or(false))
            // `force` = o usuário pediu (botão “Sugerir agora”); sem ele, o
            // serviço não gera de novo se não houve fala nova (BR-039).
            .with_force(force.unwrap_or(false)),
    )
}

/// Fixa uma sugestão já gerada na conversa (sem chamar o LLM).
///
/// Emite o evento `insight` para o overlay inserir a bolha ao vivo mesmo com a
/// janela de sugestões em foco.
#[tauri::command(async)]
pub fn suggestion_pin(
    state: State<'_, AppState>,
    session_id: String,
    mode_id: String,
    text: String,
    at_ms: i64,
) -> Result<Suggestion> {
    let pinned = state.suggestions.pin(PinSuggestionRequest {
        session_id,
        mode_id,
        text,
        at_ms,
    })?;
    state.publisher.emit_insight(&pinned);
    Ok(pinned)
}

/// Gera **e persiste** uma sugestão (botão “Sugerir” e atalho global).
///
/// Ação explícita do usuário: sempre gera, mesmo sem fala nova.
#[tauri::command(async)]
pub fn llm_insight(state: State<'_, AppState>, session_id: Option<String>) -> Result<Suggestion> {
    state
        .suggestions
        .generate(SuggestionRequest::persist(session_id).with_force(true))
}
