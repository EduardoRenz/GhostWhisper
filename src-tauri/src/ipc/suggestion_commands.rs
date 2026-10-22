//! Comandos IPC de sugestões e agentes.

use tauri::State;

use crate::domain::assistant::AssistantMode;
use crate::domain::session::SessionStatus;
use crate::domain::transcription::Speaker;
use crate::error::{GhostWhisperError, Result};
use crate::ports::manual_suggestion_context::ManualSuggestionContext;
use crate::ports::suggestion_generator::{PinSuggestionRequest, Suggestion, SuggestionRequest};
use crate::state::AppState;

/// Teto do texto selecionado para a sugestão manual.
const MANUAL_CONTEXT_MAX_CHARS: usize = 6_000;

/// Lista o catálogo efetivo de agentes: o **Padrão** (protegido) + a lista do
/// usuário, com os prompts (para editar/duplicar).
#[tauri::command(async)]
pub fn assistant_modes_list(state: State<'_, AppState>) -> Result<Vec<AssistantMode>> {
    let settings = state.settings.load()?;
    Ok(crate::application::prompts::assistant::effective_modes(
        &settings.custom_assistant_modes,
    ))
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
    manual_context: Option<ManualSuggestionContext>,
) -> Result<Suggestion> {
    state.suggestions.generate(
        SuggestionRequest::ephemeral(session_id, mode_id.unwrap_or_default())
            .with_since_ms(since_ms)
            .with_last_suggestion(last_suggestion)
            .with_extra_context(extra_context)
            .with_fast_first(fast_first.unwrap_or(false))
            // `force` = o usuário pediu (botão “Sugerir agora”); sem ele, o
            // serviço não gera de novo se não houve fala nova (BR-039).
            .with_force(force.unwrap_or(false))
            // A mensagem selecionada é um parâmetro do caso de uso (BR-041).
            .with_manual_context(manual_context),
    )
}

/// Grava a mensagem selecionada na Transcrição para a sugestão manual (BR-041).
///
/// Não chama o LLM: só valida a sessão e guarda o contexto para a janela de
/// Sugestões buscar ao abrir/atualizar.
#[tauri::command(async)]
pub fn suggestion_prepare_manual(
    state: State<'_, AppState>,
    session_id: String,
    speaker: Speaker,
    text: String,
    at_ms: i64,
) -> Result<ManualSuggestionContext> {
    let session = state
        .sessions
        .get(&session_id)?
        .ok_or_else(|| GhostWhisperError::Config("sessão não encontrada".to_string()))?;
    if session.status != SessionStatus::Active {
        return Err(GhostWhisperError::Config(
            "a sessão já foi encerrada".to_string(),
        ));
    }
    let text = text.trim();
    if text.is_empty() {
        return Err(GhostWhisperError::Config(
            "mensagem vazia: nada para sugerir".to_string(),
        ));
    }
    let text: String = text.chars().take(MANUAL_CONTEXT_MAX_CHARS).collect();
    let context = ManualSuggestionContext {
        session_id,
        speaker,
        text,
        at_ms: at_ms.max(0),
    };
    state.manual_context.put(context.clone())?;
    Ok(context)
}

/// Contexto manual atual da sessão (usado pela janela de Sugestões).
#[tauri::command(async)]
pub fn suggestion_manual_context(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<ManualSuggestionContext>> {
    state.manual_context.get(&session_id)
}

/// Limpa o contexto manual da sessão (✕/troca de sessão).
#[tauri::command(async)]
pub fn suggestion_clear_manual_context(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<()> {
    state.manual_context.clear(&session_id)
}

/// Última sugestão lembrada da sessão (cache; não chama o LLM).
#[tauri::command(async)]
pub fn suggestion_last(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<Suggestion>> {
    state.suggestions.last_suggestion(&session_id)
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
