//! Comandos IPC de configurações e chaves de API.
//!
//! A chave **nunca** é devolvida ao frontend nem registrada em logs.

use tauri::{Emitter, State};

use serde::Serialize;

use crate::domain::settings::{
    ProviderKind, Settings, MAX_AI_FONT_SCALE, MAX_FONT_SCALE, MAX_OVERLAY_OPACITY,
    MIN_AI_FONT_SCALE, MIN_FONT_SCALE, MIN_OVERLAY_OPACITY,
};
use crate::domain::shortcuts::ShortcutConflict;
use crate::error::{GhostWhisperError, Result};
use crate::ipc::window_visibility::show_above;
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
    show_above(&window).map_err(|error| GhostWhisperError::Other(error.to_string()))?;
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

/// Defaults do app (para o "Restaurar Padrões").
///
/// Vem do core porque os **agentes de fábrica** (com os contratos completos)
/// vivem na camada de aplicação — o frontend não os reproduz (BR-075).
#[tauri::command]
pub fn settings_defaults() -> Result<Settings> {
    Ok(crate::application::assistant_catalog::default_settings())
}

/// Persiste as configurações (validadas antes de gravar).
///
/// Reaplica atalhos e, quando mudam, stealth/layout do HUD; devolve os conflitos
/// de atalho encontrados.
#[tauri::command(async)]
pub fn settings_set(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Vec<ShortcutConflict>> {
    let previous = state.settings.load()?;
    // Geometria de janela é propriedade dos trackers do core (ISS-045): um
    // save de outra janela pode trazer um snapshot antigo e não deve mover o
    // notch/painéis.
    let mut settings = settings;
    settings.preserve_window_geometry(&previous);
    let layout_changed = settings.hud_layout_changed(&previous);

    state.settings.save(&settings)?;
    if settings.stealth != previous.stealth {
        state.overlay.set_stealth(settings.stealth);
    }
    // HUD (BR-062): densidade e ancoragem valem na hora — mas **só** quando
    // mudaram; salvar outra configuração não reposiciona o notch (ISS-045).
    if layout_changed {
        let density = crate::domain::hud_layout::HudDensity::parse(&settings.hud_density);
        let anchor = crate::domain::hud_layout::NotchAnchor::parse(&settings.notch_anchor);
        state.overlay.set_hud_layout(density, anchor);
        if !anchor.is_anchored() {
            // "Flutuante Livre" volta para onde o usuário deixou a **pilha**.
            if let Some((x, y)) = settings
                .overlay_position
                .filter(|(x, y)| *x >= 0 && *y >= 0)
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("overlay") {
                    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
                }
                state.overlay.set_anchor_bottom(false);
            }
        }
    }
    let conflicts = state.shortcuts.apply(&settings.shortcuts);
    // Avisa as janelas abertas (overlay/painel) para recarregarem as settings,
    // inclusive o drill-hover (BR-036). O preview garante que um save completo
    // (ex.: restaurar padrões) também atualize a opacidade global na hora.
    let _ = app.emit("hud_opacity_preview", settings.overlay_opacity);
    let _ = app.emit("font_scales_preview", FontScalePreview::of(&settings));
    let _ = app.emit("settings_changed", ());
    Ok(conflicts)
}

/// Payload do preview das escalas de fonte (BR-087).
///
/// A UI de Configurações publica a cada movimento dos sliders; todas as
/// janelas de conteúdo reaplicam sem esperar o autosave.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct FontScalePreview {
    /// Escala global da UI (0,75–1,6).
    pub font_scale: f32,
    /// Bônus do texto de IA (1,0–1,5).
    pub ai_font_scale: f32,
}

impl FontScalePreview {
    /// Lê as duas escalas de um snapshot de settings.
    pub fn of(settings: &Settings) -> Self {
        Self {
            font_scale: settings.font_scale,
            ai_font_scale: settings.ai_font_scale,
        }
    }
}

/// Nome do evento de preview das escalas de fonte (BR-087).
pub const FONT_SCALES_PREVIEW_EVENT: &str = "font_scales_preview";

/// Valida a faixa das duas escalas de fonte (BR-087).
fn validate_font_scales(font_scale: f32, ai_font_scale: f32) -> Result<()> {
    if !font_scale.is_finite() || !(MIN_FONT_SCALE..=MAX_FONT_SCALE).contains(&font_scale) {
        return Err(GhostWhisperError::Config(format!(
            "escala da fonte deve ficar entre {MIN_FONT_SCALE} e {MAX_FONT_SCALE}"
        )));
    }
    if !ai_font_scale.is_finite()
        || !(MIN_AI_FONT_SCALE..=MAX_AI_FONT_SCALE).contains(&ai_font_scale)
    {
        return Err(GhostWhisperError::Config(format!(
            "escala do texto de IA deve ficar entre {MIN_AI_FONT_SCALE} e {MAX_AI_FONT_SCALE}"
        )));
    }
    Ok(())
}

/// Publica o preview das escalas de fonte sem gravar em disco.
///
/// Os sliders de Configurações chamam a cada movimento; o valor é aplicado nas
/// janelas de conteúdo via `font_scales_preview` (BR-087).
#[tauri::command]
pub fn settings_preview_font_scales(
    app: tauri::AppHandle,
    font_scale: f32,
    ai_font_scale: f32,
) -> Result<()> {
    validate_font_scales(font_scale, ai_font_scale)?;
    let _ = app.emit(
        FONT_SCALES_PREVIEW_EVENT,
        FontScalePreview {
            font_scale,
            ai_font_scale,
        },
    );
    Ok(())
}

/// Autosave das escalas de fonte (debounced pela UI).
///
/// Grava **só** esses campos — não reaplica atalhos nem geometria — e avisa as
/// janelas. Os demais ajustes continuam no "Salvar Alterações" global.
#[tauri::command(async)]
pub fn settings_set_font_scales(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    font_scale: f32,
    ai_font_scale: f32,
) -> Result<()> {
    validate_font_scales(font_scale, ai_font_scale)?;
    let mut settings = state.settings.load()?;
    settings.font_scale = font_scale;
    settings.ai_font_scale = ai_font_scale;
    settings.validate()?;
    state.settings.save(&settings)?;
    let _ = app.emit(FONT_SCALES_PREVIEW_EVENT, FontScalePreview::of(&settings));
    let _ = app.emit("settings_changed", ());
    Ok(())
}

/// Valida a faixa da opacidade global (20 %–100 %).
fn validate_overlay_opacity(opacity: f32) -> Result<()> {
    if !opacity.is_finite() || !(MIN_OVERLAY_OPACITY..=MAX_OVERLAY_OPACITY).contains(&opacity) {
        return Err(GhostWhisperError::Config(format!(
            "opacidade das janelas deve ficar entre {MIN_OVERLAY_OPACITY} e {MAX_OVERLAY_OPACITY}"
        )));
    }
    Ok(())
}

/// Publica o preview da opacidade global sem gravar em disco.
///
/// O slider de Configurações chama a cada movimento; o valor é aplicado em
/// todas as janelas abertas via `hud_opacity_preview` (BR-062).
#[tauri::command]
pub fn settings_preview_overlay_opacity(app: tauri::AppHandle, opacity: f32) -> Result<()> {
    validate_overlay_opacity(opacity)?;
    let _ = app.emit("hud_opacity_preview", opacity);
    Ok(())
}

/// Autosave da opacidade global (debounced pela UI).
///
/// Grava apenas esse campo — não reaplica atalhos nem geometria — e avisa as
/// janelas. Os demais ajustes continuam no "Salvar Alterações" global.
#[tauri::command(async)]
pub fn settings_set_overlay_opacity(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    opacity: f32,
) -> Result<()> {
    validate_overlay_opacity(opacity)?;
    let mut settings = state.settings.load()?;
    settings.overlay_opacity = opacity;
    settings.validate()?;
    state.settings.save(&settings)?;
    let _ = app.emit("hud_opacity_preview", settings.overlay_opacity);
    let _ = app.emit("settings_changed", ());
    Ok(())
}

/// Troca o agente ativo (seletor global do overlay — BR-080).
///
/// Grava **só** esse campo: o seletor vive no overlay e está sempre montado, e
/// um snapshot completo das settings poderia sobrescrever uma edição feita em
/// outra janela enquanto ela ainda não recarregou.
#[tauri::command(async)]
pub fn settings_set_assistant_mode(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    mode_id: String,
) -> Result<()> {
    let mut settings = state.settings.load()?;
    settings.assistant_mode_id = mode_id;
    settings.validate()?;
    state.settings.save(&settings)?;
    let _ = app.emit("settings_changed", ());
    Ok(())
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
