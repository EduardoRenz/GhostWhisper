//! Comandos IPC da captura de tela (BR-055).

use std::time::{Duration, Instant};

use base64::Engine;
use tauri::{Emitter, Manager, State};

use crate::error::Result;
use crate::ipc::window_visibility::{assert_above, show_above};
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

/// Geometria de uma janela escondida durante a captura.
///
/// O `show()` de uma janela oculta pode fazer o GTK/Mutter mapeá-la em `0,0`
/// ou no centro; guardar e reaplicar a geometria mantém todas as janelas no
/// lugar em que o usuário as deixou (BR-072).
struct HiddenWindow {
    label: String,
    position: Option<tauri::PhysicalPosition<i32>>,
    size: Option<tauri::PhysicalSize<u32>>,
}

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
    // Guarda a posição da barra antes do hide: ela pode ter sido arrastada
    // menos de um ciclo antes do tracker periódico. A intenção de arraste é
    // encerrada em seguida para não aceitar posições automáticas do WM.
    crate::ipc::ask_commands::persist_ask_position(&app);
    crate::ipc::ask_commands::end_ask_drag(&app);
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
        let _ = show_above(&window);
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
/// Devolve a geometria das janelas escondidas e — para a captura não roubar o
/// foco — qual delas estava em foco (é ela que volta com o foco depois).
fn hide_app_windows(app: &tauri::AppHandle) -> (Vec<HiddenWindow>, Option<String>) {
    let mut hidden = Vec::new();
    let mut focused = None;
    for (label, window) in app.webview_windows() {
        if window.is_visible().unwrap_or(false) {
            if window.is_focused().unwrap_or(false) {
                focused = Some(label.to_string());
            }
            let snapshot = HiddenWindow {
                label: label.to_string(),
                position: window.outer_position().ok(),
                size: window.inner_size().ok(),
            };
            let _ = window.hide();
            hidden.push(snapshot);
        }
    }
    (hidden, focused)
}

/// Espera as janelas sumirem do que está desenhado.
///
/// O pedido de `hide` é assíncrono (GTK → X11 → compositor): primeiro um
/// settle fixo de [`HIDE_SETTLE_MS`] e, depois, confirma o estado do Tauri
/// (limitado por [`HIDE_SETTLE_MAX_MS`]) para não capturar cedo demais.
fn wait_for_hidden(app: &tauri::AppHandle, hidden: &[HiddenWindow]) {
    std::thread::sleep(Duration::from_millis(HIDE_SETTLE_MS));
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(HIDE_SETTLE_MAX_MS) {
        let all_hidden = hidden.iter().all(|snapshot| {
            app.get_webview_window(&snapshot.label)
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
/// ignorado logo depois do `hide` (a janela "não voltava"). A geometria é
/// reaplicada antes **e** depois do mapeamento: alguns WMs ignoram a posição
/// de uma janela oculta e a colocam em `0,0` ao mostrá-la (BR-072).
fn restore_app_windows(app: &tauri::AppHandle, hidden: Vec<HiddenWindow>, focused: Option<String>) {
    if hidden.is_empty() {
        return;
    }
    std::thread::sleep(Duration::from_millis(RESTORE_SETTLE_MS));
    for snapshot in &hidden {
        if let Some(window) = app.get_webview_window(&snapshot.label) {
            if let Some(position) = snapshot.position {
                if window.outer_position().ok() != Some(position) {
                    let _ = window.set_position(position);
                }
            }
            if let Some(size) = snapshot.size {
                if window.inner_size().ok() != Some(size) {
                    let _ = window.set_size(size);
                }
            }
            let _ = window.unminimize();
            let _ = show_above(&window);
            // Reaplica só se o WM tiver mudado a geometria ao mapear; qualquer
            // resize/move remapeia e pode derrubar o always-on-top.
            if let Some(position) = snapshot.position {
                if window.outer_position().ok() != Some(position) {
                    let _ = window.set_position(position);
                }
            }
            if let Some(size) = snapshot.size {
                if window.inner_size().ok() != Some(size) {
                    let _ = window.set_size(size);
                }
            }
            assert_above(&window);
        }
    }
    // Devolve o foco a quem o tinha (a barra de pesquisa, normalmente).
    if let Some(label) = focused {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.set_focus();
        }
    }
}
