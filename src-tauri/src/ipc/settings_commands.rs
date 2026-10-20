//! Comandos IPC de configurações e chaves de API.
//!
//! A chave **nunca** é devolvida ao frontend nem registrada em logs.

use tauri::{Emitter, State};

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

/// Esconde a janela de Configurações (mesmo comportamento do `✕`).
#[tauri::command]
pub fn settings_close(app: tauri::AppHandle) -> Result<()> {
    use tauri::Manager;

    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| GhostWhisperError::Other("janela 'settings' não encontrada".to_string()))?;
    window
        .hide()
        .map_err(|error| GhostWhisperError::Other(error.to_string()))
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
#[tauri::command(async)]
pub fn settings_get(state: State<'_, AppState>) -> Result<Settings> {
    state.settings.load()
}

/// Persiste as configurações (validadas antes de gravar).
///
/// Reaplica atalhos/stealth e devolve os conflitos de atalho encontrados.
#[tauri::command(async)]
pub fn settings_set(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Vec<ShortcutConflict>> {
    state.settings.save(&settings)?;
    state.overlay.set_stealth(settings.stealth);
    let conflicts = state.shortcuts.apply(&settings.shortcuts);
    // Avisa as janelas abertas (overlay/painel) para recarregarem as settings,
    // inclusive o drill-hover (BR-036).
    let _ = app.emit("settings_changed", ());
    Ok(conflicts)
}

/// Testa a conexão com a fonte primária (validar chave).
///
/// A validação **por fonte** vive em `ipc::ai_source_commands`; esta é a
/// atalho para a fonte primária.
#[tauri::command(async)]
pub fn settings_validate_key(state: State<'_, AppState>) -> Result<String> {
    state.llm.validate()
}

/// Rótulo do provider/modelo ativo (sem segredos).
#[tauri::command(async)]
pub fn settings_provider_label(state: State<'_, AppState>) -> Result<String> {
    state.llm.provider_label()
}

/// Informa se o armazenamento de segredos é persistente (keyring) ou de sessão.
#[tauri::command]
pub fn settings_secrets_persistent(state: State<'_, AppState>) -> bool {
    state.secrets.is_persistent()
}

/// GPU detectada e modelo local sugerido para este hardware (BR-027).
///
/// Nunca troca configuração sozinho: a UI decide se mostra a sugestão.
#[tauri::command(async)]
pub fn settings_gpu_info(state: State<'_, AppState>) -> crate::ports::gpu_probe::GpuInfo {
    state.gpu.probe()
}
