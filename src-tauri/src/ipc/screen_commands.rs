//! Comandos IPC da captura de tela (BR-055).

use std::time::{Duration, Instant};

use base64::Engine;
use tauri::{Emitter, Manager, State};

use crate::error::Result;
use crate::ports::screen_capture::CapturedImage;
use crate::state::AppState;

/// Tempo para o **compositor** tirar as janelas da tela depois do `hide()`.
///
/// `window.hide()` só enfileira o pedido; o X11/compositor leva alguns quadros
/// até a janela sumir do que está desenhado. Sem essa espera, o print sai com
/// o próprio HUD em cima (foi o que aconteceu nos testes).
const HIDE_SETTLE_MS: u64 = 200;
/// Teto de espera confirmando o `unmap` (além do settle fixo).
const HIDE_SETTLE_MAX_MS: u64 = 600;
/// Respiro antes de restaurar (o `show` logo após o `hide` pode ser ignorado).
const RESTORE_SETTLE_MS: u64 = 60;

/// Imagem capturada (em base64, para o frontend montar a data URL).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CapturedImageDto {
    /// Tipo MIME (ex.: `image/jpeg`).
    pub mime: String,
    /// Dados em base64 (sem o prefixo `data:`).
    pub data: String,
    pub width: u32,
    pub height: u32,
}

impl From<CapturedImage> for CapturedImageDto {
    fn from(image: CapturedImage) -> Self {
        let data = base64::engine::general_purpose::STANDARD.encode(&image.data);
        Self {
            mime: image.mime,
            data,
            width: image.width,
            height: image.height,
        }
    }
}

/// Captura a tela e deixa o print **pendente** (anexado à pergunta).
///
/// As janelas do app são **escondidas durante a captura** e restauradas
/// depois — o print não sai com o HUD em cima (paridade com o
/// `protectPerssuaWindowsForCapture` do Perssua).
///
/// O print **não é enviado**: ele fica em [`PendingCapture`] e a prévia no
/// canto superior direito aparece. O envio só acontece quando o usuário
/// confirma a pergunta (BR-055).
#[tauri::command(async)]
pub fn screen_capture(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<CapturedImageDto> {
    let (hidden, focused) = hide_app_windows(&app);
    wait_for_hidden(&app, &hidden);
    let captured = state.screen.capture();
    restore_app_windows(&app, hidden, focused);

    let image = captured?;
    state.capture.set(image.clone())?;
    show_preview(&app);
    notify_capture_changed(&app);
    Ok(image.into())
}

/// Print pendente (a prévia lê daqui ao abrir).
#[tauri::command(async)]
pub fn capture_pending(state: State<'_, AppState>) -> Result<Option<CapturedImageDto>> {
    Ok(state.capture.get()?.map(Into::into))
}

/// Descarta o print pendente e fecha a prévia.
#[tauri::command(async)]
pub fn capture_clear(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<()> {
    state.capture.clear()?;
    hide_preview(&app);
    notify_capture_changed(&app);
    Ok(())
}

/// Mostra a janela da prévia (canto superior direito).
fn show_preview(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("capture") {
        let _ = window.show();
        let _ = app.emit_to("capture", "capture_pending", ());
    }
}

/// Fecha a janela da prévia.
fn hide_preview(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("capture") {
        let _ = window.hide();
    }
}

/// Avisa a barra de pesquisa que o anexo mudou.
fn notify_capture_changed(app: &tauri::AppHandle) {
    let _ = app.emit("capture_changed", ());
}

/// Esconde as janelas visíveis do app.
///
/// Devolve os labels escondidos e — para a captura não roubar o foco — qual
/// delas estava em foco (é ela que volta com o foco depois).
fn hide_app_windows(app: &tauri::AppHandle) -> (Vec<String>, Option<String>) {
    let mut hidden = Vec::new();
    let mut focused = None;
    for (label, window) in app.webview_windows() {
        if window.is_visible().unwrap_or(false) {
            if window.is_focused().unwrap_or(false) {
                focused = Some(label.to_string());
            }
            let _ = window.hide();
            hidden.push(label.to_string());
        }
    }
    (hidden, focused)
}

/// Espera as janelas sumirem do que está desenhado.
///
/// O pedido de `hide` é assíncrono (GTK → X11 → compositor): primeiro um
/// settle fixo de [`HIDE_SETTLE_MS`] e, depois, confirma o estado do Tauri
/// (limitado por [`HIDE_SETTLE_MAX_MS`]) para não capturar cedo demais.
fn wait_for_hidden(app: &tauri::AppHandle, labels: &[String]) {
    std::thread::sleep(Duration::from_millis(HIDE_SETTLE_MS));
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(HIDE_SETTLE_MAX_MS) {
        let all_hidden = labels.iter().all(|label| {
            app.get_webview_window(label)
                .and_then(|window| window.is_visible().ok())
                .map(|visible| !visible)
                .unwrap_or(true)
        });
        if all_hidden {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Devolve as janelas ao estado anterior.
///
/// `unminimize` antes do `show` cobre o caso de o WM ter minimizado a janela
/// ao escondê-la; um respiro curto entre esconder/mostrar evita o `show` ser
/// ignorado logo depois do `hide` (a janela "não voltava").
fn restore_app_windows(app: &tauri::AppHandle, hidden: Vec<String>, focused: Option<String>) {
    if hidden.is_empty() {
        return;
    }
    std::thread::sleep(Duration::from_millis(RESTORE_SETTLE_MS));
    for label in &hidden {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.unminimize();
            let _ = window.show();
        }
    }
    // Devolve o foco a quem o tinha (a barra de pesquisa, normalmente).
    if let Some(label) = focused {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.set_focus();
        }
    }
}
