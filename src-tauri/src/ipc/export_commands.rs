//! Comandos IPC de exportação de sessão (Markdown).

use std::path::PathBuf;

use tauri::State;

use crate::application::markdown_export::render_markdown_with_questions;
use crate::application::session_timeline::build_timeline;
use crate::error::{GhostWhisperError, Result};
use crate::state::AppState;

/// Monta o Markdown da sessão (usado para copiar e para salvar).
fn build_markdown(state: &AppState, session_id: &str) -> Result<String> {
    let session = state
        .sessions
        .get(session_id)?
        .ok_or_else(|| GhostWhisperError::Config(format!("sessão {session_id} não encontrada")))?;
    let segments = state.transcriptions.list_by_session(session_id)?;
    let messages = state.messages.list_by_session(session_id)?;
    let entries = build_timeline(&segments, &messages);
    let questions = state.journal.list_by_session(session_id)?;
    // Uma sessão em andamento usa "agora" para a duração.
    Ok(render_markdown_with_questions(
        &session,
        &entries,
        state.clock.now_ms(),
        &questions,
    ))
}

/// Exporta a sessão como Markdown e devolve o texto (para copiar).
#[tauri::command(async)]
pub fn session_export_markdown(state: State<'_, AppState>, id: String) -> Result<String> {
    build_markdown(&state, &id)
}

/// Resolve o caminho final do arquivo (`.md` quando faltar, caminho vazio recusado).
fn resolve_target(path: &str) -> Result<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(GhostWhisperError::Config(
            "caminho do arquivo vazio".to_string(),
        ));
    }
    let mut target = PathBuf::from(trimmed);
    if target.extension().is_none() {
        target.set_extension("md");
    }
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(target)
}

/// Exporta a sessão e grava o Markdown no caminho escolhido.
///
/// Retorna o caminho final (ganha sufixo `.md` quando faltar).
#[tauri::command(async)]
pub fn session_save_markdown(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<String> {
    let markdown = build_markdown(&state, &id)?;
    let target = resolve_target(&path)?;
    std::fs::write(&target, markdown)?;
    tracing::info!(path = %target.display(), "sessão exportada em Markdown");
    Ok(target.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_the_markdown_extension_when_missing() {
        let target = resolve_target("/tmp/relatorio").expect("resolve");
        assert_eq!(target, PathBuf::from("/tmp/relatorio.md"));

        let keep = resolve_target("/tmp/relatorio.md").expect("resolve");
        assert_eq!(keep, PathBuf::from("/tmp/relatorio.md"));
    }

    #[test]
    fn rejects_an_empty_path() {
        let error = resolve_target("   ").unwrap_err().to_string();
        assert!(error.contains("vazio"), "{error}");
    }

    #[test]
    fn creates_the_parent_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("exports").join("sessao.md");
        let target = resolve_target(&nested.to_string_lossy()).expect("resolve");
        std::fs::write(&target, "# ok").expect("write");
        assert_eq!(std::fs::read_to_string(target).expect("read"), "# ok");
    }
}
