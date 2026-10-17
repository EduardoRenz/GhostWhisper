//! Comandos IPC de atalhos globais.

use serde::Serialize;
use tauri::State;

use crate::domain::shortcuts::{ShortcutAction, ShortcutBinding, ShortcutConflict};
use crate::error::Result;
use crate::state::AppState;

/// Catálogo de ações de atalho (para a UI).
#[derive(Debug, Serialize)]
pub struct ShortcutInfo {
    pub action: &'static str,
    pub label: &'static str,
    pub default_accelerator: &'static str,
}

/// Lista as ações disponíveis e seus padrões.
#[tauri::command]
pub fn shortcuts_catalog() -> Vec<ShortcutInfo> {
    ShortcutAction::all()
        .iter()
        .map(|action| ShortcutInfo {
            action: action.as_str(),
            label: action.label(),
            default_accelerator: action.default_accelerator(),
        })
        .collect()
}

/// Atalhos configurados (das settings).
#[tauri::command]
pub fn shortcuts_get(state: State<'_, AppState>) -> Result<Vec<ShortcutBinding>> {
    Ok(state.settings.load()?.shortcuts)
}

/// Atalhos efetivamente registrados no sistema.
#[tauri::command]
pub fn shortcuts_active(state: State<'_, AppState>) -> Vec<ShortcutBinding> {
    state.shortcuts.active()
}

/// Persiste e reaplica os atalhos; devolve os conflitos encontrados.
#[tauri::command]
pub fn shortcuts_set(
    state: State<'_, AppState>,
    bindings: Vec<ShortcutBinding>,
) -> Result<Vec<ShortcutConflict>> {
    let mut settings = state.settings.load()?;
    settings.shortcuts = bindings;
    settings.validate()?;
    state.settings.save(&settings)?;
    Ok(state.shortcuts.apply(&settings.shortcuts))
}
