//! Comandos IPC da janela **Perguntar** (a barra de pesquisa, BR-052).
//!
//! A barra é uma janela própria, aberta pela **lupa do notch** (disponível
//! independente de haver transcrição) e pelo atalho global.

use tauri::{Emitter, Manager};

use crate::error::{GhostWhisperError, Result};

/// Janela da barra "Perguntar".
pub const ASK_WINDOW: &str = "ask";

/// Largura lógica da barra de pesquisa (janela `ask`).
pub const ASK_WIDTH: f64 = 1040.0;

/// Distância da barra até o topo da tela (lógica).
const ASK_TOP_MARGIN: f64 = 12.0;

/// Centraliza a barra no **topo** do monitor principal na primeira execução.
///
/// Depois que existe `ask_position`, a barra é restaurada nessa posição. Abrir,
/// fechar, capturar a tela ou salvar settings **nunca** a recentraliza
/// (BR-052/BR-072).
pub fn center_ask_window(app: &tauri::AppHandle) -> Option<(i32, i32)> {
    let window = app.get_webview_window(ASK_WINDOW)?;
    let monitor = app.primary_monitor().ok().flatten()?;
    let size = monitor.size();
    let scale = monitor.scale_factor();
    let width = (ASK_WIDTH * scale).round() as i32;
    let margin = (ASK_TOP_MARGIN * scale).round() as i32;
    let x = (size.width as i32 - width) / 2;
    let _ = window.set_position(tauri::PhysicalPosition::new(x, margin));
    Some((x, margin))
}

/// Mantém `ask_position` em sincronia com a posição real da barra.
fn save_ask_position(app: &tauri::AppHandle, position: Option<(i32, i32)>) {
    let Some(position) = position else {
        return;
    };
    let state = app.state::<crate::state::AppState>();
    let Ok(mut settings) = state.settings.load() else {
        return;
    };
    if settings.ask_position != Some(position) {
        settings.ask_position = Some(position);
        let _ = state.settings.save(&settings);
    }
}

/// Lê a posição salva para reaplicá-la ao mostrar a janela.
fn saved_ask_position(app: &tauri::AppHandle) -> Option<(i32, i32)> {
    let state = app.state::<crate::state::AppState>();
    state
        .settings
        .load()
        .ok()
        .and_then(|settings| settings.ask_position)
}

fn place_ask_window(window: &tauri::WebviewWindow, position: Option<(i32, i32)>) {
    if let Some((x, y)) = position {
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
}

/// Persiste a posição atual antes de esconder/capturar a barra.
///
/// O tracker periódico também cobre a permanência, mas fechar logo depois de
/// arrastar não pode perder o último ponto (BR-072).
pub fn persist_ask_position(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(ASK_WINDOW) else {
        return;
    };
    if !window.is_visible().unwrap_or(false) {
        return;
    }
    if let Ok(position) = window.outer_position() {
        save_ask_position(app, Some((position.x, position.y)));
    }
}

/// Restaura a posição persistida da barra (ou a centraliza na primeira vez).
///
/// Chamado **só no boot**. A posição salva é a última em que o usuário
/// arrastou a barra; nenhuma abertura posterior a move (BR-072).
pub fn restore_ask_window_position(app: &tauri::AppHandle, saved: Option<(i32, i32)>) {
    let Some(window) = app.get_webview_window(ASK_WINDOW) else {
        return;
    };
    match saved {
        Some((x, y)) => {
            let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        }
        None => {
            center_ask_window(app);
        }
    }
}

/// Mostra a barra na **última posição salva** e devolve o foco ao campo.
///
/// O GTK/Mutter pode remapear uma janela oculta em `0,0` ao chamar `show`; por
/// isso a posição é reaplicada antes e depois do mapeamento. O valor é sempre o
/// último ponto do usuário, nunca uma centralização (BR-072). O evento
/// `ask_focus` garante o cursor no campo; o atalho global depende disso.
pub fn show_ask_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(ASK_WINDOW) else {
        return;
    };
    let was_hidden = !window.is_visible().unwrap_or(false);
    if was_hidden {
        let saved = saved_ask_position(app);
        let target = saved.or_else(|| center_ask_window(app));
        place_ask_window(&window, target);
        let _ = window.show();
        place_ask_window(&window, target);
        if saved.is_none() {
            // Primeira colocação: guarda o centro para as próximas aberturas.
            save_ask_position(app, target);
        }
    }
    let _ = window.set_focus();
    let _ = app.emit_to(ASK_WINDOW, "ask_focus", ());
}

/// Abre (ou foca) a barra de pesquisa.
#[tauri::command]
pub fn ask_open(app: tauri::AppHandle) -> Result<()> {
    if app.get_webview_window(ASK_WINDOW).is_none() {
        return Err(GhostWhisperError::Other(
            "janela 'ask' não encontrada".to_string(),
        ));
    }
    show_ask_window(&app);
    Ok(())
}

/// Fecha a barra de pesquisa.
#[tauri::command]
pub fn ask_hide(app: tauri::AppHandle) -> Result<()> {
    let window = app
        .get_webview_window(ASK_WINDOW)
        .ok_or_else(|| GhostWhisperError::Other("janela 'ask' não encontrada".to_string()))?;
    persist_ask_position(&app);
    let _ = window.hide();
    Ok(())
}

/// A barra está visível? (fallback quando o evento se perde)
#[tauri::command]
pub fn ask_is_open(app: tauri::AppHandle) -> Result<bool> {
    let window = app
        .get_webview_window(ASK_WINDOW)
        .ok_or_else(|| GhostWhisperError::Other("janela 'ask' não encontrada".to_string()))?;
    Ok(window.is_visible().unwrap_or(false))
}

/// Foca o campo da barra (emitido pelo atalho global).
#[tauri::command]
pub fn ask_focus(app: tauri::AppHandle) -> Result<()> {
    let _ = app.emit_to(ASK_WINDOW, "ask_focus", ());
    Ok(())
}
