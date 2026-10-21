//! Comandos IPC da janela **Perguntar** (a barra de pesquisa, BR-052).
//!
//! A barra é uma janela própria, aberta pela **lupa do notch** (disponível
//! independente de haver transcrição) e pelo atalho global.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::{Emitter, Manager, State};

use crate::domain::settings::AskPositionSource;
use crate::error::{GhostWhisperError, Result};
use crate::ipc::window_visibility::{assert_above, show_above};

/// Janela da barra "Perguntar".
pub const ASK_WINDOW: &str = "ask";

/// Largura lógica da barra de pesquisa (janela `ask`).
pub const ASK_WIDTH: f64 = 1040.0;

/// Distância da barra até o topo da tela (lógica).
const ASK_TOP_MARGIN: f64 = 12.0;

/// Tempo sem movimento que encerra a intenção de arraste (ms).
///
/// A posição só pode ser persistida enquanto o arraste da alça está ativo;
/// depois disso, qualquer posição do WM é ignorada (BR-072/ISS-057).
const ASK_DRAG_IDLE_MS: u64 = 2_500;

/// Intenção de arraste da barra, gerenciada pelo Tauri (sem estado global).
#[derive(Default)]
pub struct AskDragState {
    active: AtomicBool,
    last_activity_ms: AtomicU64,
}

impl AskDragState {
    /// Marca o início de um arraste explícito.
    pub fn begin(&self, now_ms: u64) {
        self.active.store(true, Ordering::Relaxed);
        self.last_activity_ms.store(now_ms, Ordering::Relaxed);
    }

    /// Renova a janela de intenção enquanto a posição muda.
    pub fn touch(&self, now_ms: u64) {
        self.last_activity_ms.store(now_ms, Ordering::Relaxed);
    }

    /// Encerra a intenção (fechar/capturar ou falha ao iniciar o arraste).
    pub fn end(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    /// `true` enquanto o usuário está arrastando (ou acabou de arrastar).
    pub fn is_active(&self, now_ms: u64) -> bool {
        if !self.active.load(Ordering::Relaxed) {
            return false;
        }
        let idle = now_ms.saturating_sub(self.last_activity_ms.load(Ordering::Relaxed));
        if idle > ASK_DRAG_IDLE_MS {
            self.active.store(false, Ordering::Relaxed);
            return false;
        }
        true
    }
}

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

/// Tenta persistir uma posição da barra, respeitando a origem.
fn save_ask_position(
    app: &tauri::AppHandle,
    position: Option<(i32, i32)>,
    source: AskPositionSource,
) -> bool {
    let Some(position) = position else {
        return false;
    };
    let state = app.state::<crate::state::AppState>();
    let Ok(mut settings) = state.settings.load() else {
        return false;
    };
    if !settings.apply_ask_position(position, source) {
        return false;
    }
    let _ = state.settings.save(&settings);
    true
}

/// Observa a posição atual; só persiste com intenção de arraste.
///
/// O tracker periódico chama isto a cada ciclo. Quando o WM move a janela
/// sozinho, `drag_source` retorna `Automatic` e a posição é descartada.
pub fn observe_ask_position(app: &tauri::AppHandle, position: (i32, i32)) -> bool {
    let source = drag_source(app);
    save_ask_position(app, Some(position), source)
}

fn drag_source(app: &tauri::AppHandle) -> AskPositionSource {
    let drag = app.state::<AskDragState>();
    let now = app.state::<crate::state::AppState>().clock.now_ms();
    if drag.is_active(now as u64) {
        AskPositionSource::UserDrag
    } else {
        AskPositionSource::Automatic
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
    let source = drag_source(app);
    if let Ok(position) = window.outer_position() {
        save_ask_position(app, Some((position.x, position.y)), source);
    }
}

/// Encerra a intenção de arraste (chamado ao fechar/capturar).
pub fn end_ask_drag(app: &tauri::AppHandle) {
    app.state::<AskDragState>().end();
}

/// Reaplica a posição salva depois que o WM termina de mapear a janela.
///
/// Alguns WMs colocam a janela em `0,0` *depois* do `show`; uma reafirmação
/// curta garante o último ponto do usuário. Se ele já começou outro arraste,
/// a reafirmação é cancelada para não brigar com o ponteiro (BR-072).
fn reassert_ask_position_later(app: tauri::AppHandle, target: (i32, i32)) {
    std::thread::spawn(move || {
        for delay in [140_u64, 380] {
            std::thread::sleep(Duration::from_millis(delay));
            let Some(window) = app.get_webview_window(ASK_WINDOW) else {
                return;
            };
            if !window.is_visible().unwrap_or(false) {
                return;
            }
            if drag_source(&app) == AskPositionSource::UserDrag {
                return;
            }
            if let Ok(current) = window.outer_position() {
                if (current.x, current.y) != target {
                    let _ = window.set_position(tauri::PhysicalPosition::new(target.0, target.1));
                }
            }
            assert_above(&window);
        }
    });
}

/// Inicia o arraste explícito da barra e marca a intenção para o tracker.
#[tauri::command]
pub fn ask_start_drag(
    app: tauri::AppHandle,
    state: State<'_, AskDragState>,
    window: tauri::WebviewWindow,
) -> Result<()> {
    let now = app.state::<crate::state::AppState>().clock.now_ms() as u64;
    state.begin(now);
    if let Err(error) = window.start_dragging() {
        state.end();
        return Err(GhostWhisperError::Other(format!(
            "falha ao arrastar a barra: {error}"
        )));
    }
    Ok(())
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
    let saved = saved_ask_position(app);
    let target = if was_hidden {
        saved.or_else(|| center_ask_window(app))
    } else {
        saved
    };
    if was_hidden {
        place_ask_window(&window, target);
        let _ = show_above(&window);
        place_ask_window(&window, target);
        if saved.is_none() {
            // Primeira colocação: guarda o centro para as próximas aberturas.
            save_ask_position(app, target, AskPositionSource::InitialPlacement);
        }
    }
    assert_above(&window);
    if let Some(target) = target {
        reassert_ask_position_later(app.clone(), target);
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
    end_ask_drag(&app);
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

#[cfg(test)]
mod tests {
    use super::AskDragState;

    /// ISS-057: depois do arraste, o estado expira e não aceita mais
    /// posições automáticas do WM.
    #[test]
    fn drag_intent_expires_after_idle() {
        let state = AskDragState::default();
        assert!(!state.is_active(1_000));

        state.begin(1_000);
        assert!(state.is_active(1_500));
        state.touch(3_000);
        assert!(state.is_active(5_000));

        // Sem novos movimentos, a posição automática não pode ser persistida.
        assert!(!state.is_active(5_501));
        assert!(!state.is_active(5_600));
    }

    #[test]
    fn ending_drag_disables_persistence_immediately() {
        let state = AskDragState::default();
        state.begin(1_000);
        assert!(state.is_active(1_100));
        state.end();
        assert!(!state.is_active(1_100));
    }
}
