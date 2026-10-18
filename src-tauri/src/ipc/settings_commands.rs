//! Comandos IPC de configurações e chaves de API.
//!
//! A chave **nunca** é devolvida ao frontend nem registrada em logs.

use tauri::State;

use crate::domain::settings::{ProviderKind, Settings};
use crate::domain::shortcuts::ShortcutConflict;
use crate::error::{GhostWhisperError, Result};
use crate::state::AppState;

/// Abre (ou traz à frente) a janela de configurações.
#[tauri::command]
pub fn settings_open(app: tauri::AppHandle) -> Result<()> {
    use tauri::Manager;

    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| GhostWhisperError::Other("janela 'settings' não encontrada".to_string()))?;
    // A janela é apenas escondida ao fechar; basta desminimizar e reexibir.
    let _ = window.unminimize();
    window
        .show()
        .map_err(|error| GhostWhisperError::Other(error.to_string()))?;
    window
        .set_focus()
        .map_err(|error| GhostWhisperError::Other(error.to_string()))?;
    Ok(())
}

/// Lista os providers suportados (para a UI).
#[tauri::command]
pub fn settings_providers() -> Vec<&'static str> {
    ProviderKind::all()
        .iter()
        .map(|kind| kind.as_str())
        .collect()
}

/// Retorna as configurações atuais (sem segredos).
#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<Settings> {
    state.settings.load()
}

/// Persiste as configurações (validadas antes de gravar).
///
/// Reaplica atalhos/stealth e devolve os conflitos de atalho encontrados.
#[tauri::command]
pub fn settings_set(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Vec<ShortcutConflict>> {
    state.settings.save(&settings)?;
    state.overlay.set_stealth(settings.stealth);
    Ok(state.shortcuts.apply(&settings.shortcuts))
}

/// Grava a chave do provider ativo no armazenamento de segredos.
#[tauri::command]
pub fn settings_set_provider_key(state: State<'_, AppState>, key: String) -> Result<()> {
    let settings = state.settings.load()?;
    state.secrets.set(&settings.provider.secret_key(), &key)
}

/// Indica se há chave configurada para o provider ativo.
#[tauri::command]
pub fn settings_has_provider_key(state: State<'_, AppState>) -> Result<bool> {
    let settings = state.settings.load()?;
    let present = state
        .secrets
        .get(&settings.provider.secret_key())?
        .is_some_and(|value| !value.trim().is_empty());
    Ok(present)
}

/// Remove a chave do provider ativo.
#[tauri::command]
pub fn settings_clear_provider_key(state: State<'_, AppState>) -> Result<()> {
    let settings = state.settings.load()?;
    state.secrets.delete(&settings.provider.secret_key())
}

/// Testa a conexão com o provider (validar chave).
#[tauri::command]
pub fn settings_validate_key(state: State<'_, AppState>) -> Result<String> {
    state.llm.validate()
}

/// Rótulo do provider/modelo ativo (sem segredos).
#[tauri::command]
pub fn settings_provider_label(state: State<'_, AppState>) -> Result<String> {
    state.llm.provider_label()
}

/// Versão mascarada da chave ativa (`••••••••·abcd`) para indicar presença.
///
/// Nunca devolve o valor bruto: só os últimos 4 caracteres, mesma regra do
/// OpenAI/OpenRouter quando exibem chaves salvas.
#[tauri::command]
pub fn settings_key_preview(state: State<'_, AppState>) -> Result<String> {
    let settings = state.settings.load()?;
    let masked = state
        .secrets
        .get(&settings.provider.secret_key())?
        .map(|value| crate::domain::settings::redact_secret(&value))
        .unwrap_or_default();
    Ok(masked)
}

/// Informa se o armazenamento de segredos é persistente (keyring) ou de sessão.
#[tauri::command]
pub fn settings_secrets_persistent(state: State<'_, AppState>) -> bool {
    state.secrets.is_persistent()
}
