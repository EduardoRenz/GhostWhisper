//! Ativação de janelas com timestamp válido no X11.
//!
//! O `set_focus` do Tauri termina em `gtk_window_present_with_time(GDK_CURRENT_TIME)`,
//! isto é, **timestamp 0**. Em Mutter/X11 a ativação de um app inativo com
//! timestamp 0 é recusada (focus stealing prevention) — o mesmo mecanismo que
//! fazia o arraste precisar de dois cliques (ISS-063). Aqui usamos o tempo
//! atual do servidor X11, que o WM aceita como pedido posterior à última
//! interação do usuário (ISS-064/BR-052).

use tauri::WebviewWindow;

/// Repetir a ativação depois do mapeamento?
///
/// Só faz sentido quando a janela está visível e ainda não recebeu o foco do
/// teclado; uma janela oculta é tratada pelo `show` e uma já focada não precisa
/// de novo pedido ao WM.
pub fn should_refocus(visible: bool, focused: bool) -> bool {
    visible && !focused
}

#[cfg(target_os = "linux")]
mod imp {
    use gdkx11::X11Window;
    use gtk::prelude::*;

    /// Apresenta a janela com o timestamp do servidor X11.
    ///
    /// Devolve `false` quando não há janela GTK/X11 (ex.: Wayland), para o
    /// chamador usar o caminho padrão do Tauri.
    pub fn present(window: &tauri::WebviewWindow) -> bool {
        let Ok(gtk_window) = window.gtk_window() else {
            return false;
        };
        match gtk_window
            .window()
            .and_then(|gdk| gdk.downcast::<X11Window>().ok())
        {
            Some(x11) => {
                let timestamp = gdkx11::functions::x11_get_server_time(&x11);
                gtk_window.present_with_time(timestamp);
            }
            // Wayland: o compositor conduz a ativação; não há timestamp X11.
            None => gtk_window.present(),
        }
        true
    }
}

/// Traz a janela para frente e entrega o foco do teclado.
///
/// Sempre executa no loop principal do GTK: o chamador pode estar na thread do
/// atalho global, e o `present` precisa de um timestamp do servidor X11.
pub fn focus_window(app: &tauri::AppHandle, window: &WebviewWindow) {
    let window = window.clone();
    let _ = app.run_on_main_thread(move || {
        #[cfg(target_os = "linux")]
        {
            if imp::present(&window) {
                return;
            }
        }
        let _ = window.set_focus();
    });
}

#[cfg(test)]
mod tests {
    use super::should_refocus;

    /// ISS-064: a janela já aberta e sem foco precisa de nova ativação; os
    /// outros estados não devem gerar pedidos extras ao WM.
    #[test]
    fn refocus_only_when_visible_and_unfocused() {
        assert!(should_refocus(true, false));
        assert!(!should_refocus(true, true));
        assert!(!should_refocus(false, false));
        assert!(!should_refocus(false, true));
    }
}
