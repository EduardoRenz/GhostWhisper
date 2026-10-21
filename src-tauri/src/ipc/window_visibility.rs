//! Helpers de exibição e empilhamento das janelas (BR-073).
//!
//! O GTK/Mutter pode perder `_NET_WM_STATE_ABOVE` quando uma janela é
//! desmapeada/remapada (`hide`/`show`) ou redimensionada. Toda exibição deve
//! reafirmar `alwaysOnTop` **depois** da última mudança de geometria.

use tauri::WebviewWindow;

/// Reafirma que a janela fica acima das demais.
pub fn assert_above(window: &WebviewWindow) {
    let _ = window.set_always_on_top(true);
}

/// Mostra a janela e reafirma o empilhamento antes e depois do mapeamento.
pub fn show_above(window: &WebviewWindow) -> tauri::Result<()> {
    assert_above(window);
    window.show()?;
    assert_above(window);
    Ok(())
}
