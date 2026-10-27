//! Comandos IPC do model manager do classificador (BR-099).
//!
//! Nenhum comando baixa nada sem uma chamada explícita. `install` instala e
//! ativa a versão pedida numa única ação do usuário; `remove` nunca remove a
//! versão ativa (erro tipado no manager).

use serde::Serialize;
use tauri::{Manager, State};

use crate::domain::classifier_model::{ClassifierModelCatalog, InstalledClassifierModel};
use crate::error::{GhostWhisperError, Result};
use crate::ports::classifier_model_manager::ClassifierModelError;
use crate::state::AppState;

fn to_error(error: ClassifierModelError) -> GhostWhisperError {
    GhostWhisperError::Other(error.to_string())
}

/// Estado do backend Laya **neste build** (a UI não adivinha feature/modelo).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClassifierBackendStatus {
    /// `true` quando o binário foi compilado com `--features laya`.
    pub laya_feature: bool,
    /// `true` quando há uma versão ativa no model manager.
    pub model_active: bool,
    /// `true` quando há override de dev, versão ativa ou modelo empacotado.
    pub model_configured: bool,
    /// `true` quando o AppImage/deb traz o modelo nos resources.
    pub model_bundled: bool,
}

/// Informa se o engine Laya existe neste build e se há modelo configurado.
#[tauri::command(async)]
pub fn classifier_backend_status(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ClassifierBackendStatus> {
    let active = state.classifier_models.active();
    let settings = state.settings.load()?;
    let bundled = app
        .path()
        .resource_dir()
        .ok()
        .map(|dir| {
            dir.join("laya")
                .join("multilingual")
                .join("model.onnx")
                .is_file()
        })
        .unwrap_or(false);
    Ok(ClassifierBackendStatus {
        laya_feature: cfg!(feature = "laya"),
        model_active: active.is_some(),
        model_configured: !settings.suggestion_classifier_model_path.trim().is_empty()
            || active.is_some()
            || bundled,
        model_bundled: bundled,
    })
}

/// Catálogo publicado (`catalog.json`). Só lê metadados; não baixa o modelo.
#[tauri::command(async)]
pub fn classifier_model_catalog(state: State<'_, AppState>) -> Result<ClassifierModelCatalog> {
    state.classifier_models.catalog().map_err(to_error)
}

/// Versões instaladas no disco (com a flag de ativa).
#[tauri::command(async)]
pub fn classifier_model_installed(
    state: State<'_, AppState>,
) -> Result<Vec<InstalledClassifierModel>> {
    state.classifier_models.installed().map_err(to_error)
}

/// Versão ativa (ponteiro `current`), se houver.
#[tauri::command(async)]
pub fn classifier_model_active(
    state: State<'_, AppState>,
) -> Result<Option<InstalledClassifierModel>> {
    Ok(state.classifier_models.active())
}

/// Baixa, verifica e **ativa** uma versão do catálogo.
#[tauri::command(async)]
pub fn classifier_model_install(
    state: State<'_, AppState>,
    model_id: String,
    version: String,
) -> Result<InstalledClassifierModel> {
    state
        .classifier_models
        .install(&model_id, &version)
        .map_err(to_error)?;
    state
        .classifier_models
        .activate(&model_id, &version)
        .map_err(to_error)
}

/// Ativa uma versão já instalada e verificada.
#[tauri::command(async)]
pub fn classifier_model_activate(
    state: State<'_, AppState>,
    model_id: String,
    version: String,
) -> Result<InstalledClassifierModel> {
    state
        .classifier_models
        .activate(&model_id, &version)
        .map_err(to_error)
}

/// Remove uma versão instalada (a ativa é protegida).
#[tauri::command(async)]
pub fn classifier_model_remove(
    state: State<'_, AppState>,
    model_id: String,
    version: String,
) -> Result<()> {
    state
        .classifier_models
        .remove(&model_id, &version)
        .map_err(to_error)
}
