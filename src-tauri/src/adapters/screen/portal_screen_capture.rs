//! Captura de tela no **Wayland** via XDG Desktop Portal.
//!
//! Em Wayland o `GetImage` do X11 não vê as janelas nativas e o modo
//! não-interativo do portal exige um diálogo de permissão associado a uma janela
//! Wayland (o app roda em XWayland para manter o notch no topo). Por isso usamos
//! o modo **interativo**: o GNOME mostra a UI de captura, o usuário confirma a
//! área/tela e o portal devolve o arquivo. Reutilizamos a mesma escala/JPEG do
//! adaptador X11.

use std::path::PathBuf;

use ashpd::desktop::screenshot::Screenshot;

use crate::error::{GhostWhisperError, Result};
use crate::ports::screen_capture::{CapturedImage, ScreenCapture};

use super::x11_screen_capture::encode_jpeg;

/// Captura de tela pelo portal do Wayland (UI interativa do compositor).
pub struct PortalScreenCapture;

impl PortalScreenCapture {
    /// Cria o capturador (nenhuma chamada ao portal é feita aqui).
    pub fn new() -> Self {
        Self
    }
}

impl Default for PortalScreenCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenCapture for PortalScreenCapture {
    fn capture(&self) -> Result<CapturedImage> {
        // O portal usa D-Bus assíncrono. A porta é síncrona e o comando IPC já
        // roda fora da main thread: uma thread própria com runtime evita criar
        // um runtime dentro do runtime do Tauri e não trava a UI enquanto o
        // usuário confirma a captura.
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| {
                    GhostWhisperError::Other(format!("runtime do portal indisponível: {error}"))
                })
                .and_then(|runtime| runtime.block_on(capture_via_portal()));
            let _ = tx.send(result);
        });
        rx.recv().map_err(|_| {
            GhostWhisperError::Other("captura de tela cancelada pelo portal".to_string())
        })?
    }
}

/// Pede a captura ao portal e comprime o resultado em JPEG.
async fn capture_via_portal() -> Result<CapturedImage> {
    let connection = crate::adapters::system::portal::connection().await?;
    let response = Screenshot::request()
        .connection(Some(connection))
        .interactive(true)
        .modal(false)
        .send()
        .await
        .map_err(portal_error)?
        .response()
        .map_err(portal_error)?;

    let path = uri_to_path(response.uri().as_str()).ok_or_else(|| {
        GhostWhisperError::Other("o portal devolveu uma captura sem arquivo local".to_string())
    })?;
    let bytes = std::fs::read(&path).map_err(|error| {
        GhostWhisperError::Other(format!("captura de tela (portal Wayland): {error}"))
    })?;
    // O print é um anexo temporário da pergunta; não deixa uma cópia na pasta
    // de imagens do usuário.
    let _ = std::fs::remove_file(&path);

    let image = image::load_from_memory(&bytes).map_err(|error| {
        GhostWhisperError::Other(format!("captura de tela (portal Wayland): {error}"))
    })?;
    let rgb = image.to_rgb8();
    let (data, width, height) = encode_jpeg(rgb.as_raw(), rgb.width(), rgb.height())?;
    tracing::info!(width, height, "captura via portal Wayland");
    Ok(CapturedImage {
        mime: "image/jpeg".to_string(),
        data,
        width,
        height,
    })
}

/// Mensagem amigável para cancelamento/recusa do portal.
fn portal_error(error: ashpd::Error) -> GhostWhisperError {
    GhostWhisperError::Other(format!(
        "captura de tela cancelada ou indisponível no portal do GNOME ({error})"
    ))
}

/// Converte o `file://` do portal em caminho local.
fn uri_to_path(uri: &str) -> Option<PathBuf> {
    url::Url::parse(uri).ok()?.to_file_path().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_portal_file_uri() {
        assert_eq!(
            uri_to_path("file:///tmp/captura.png"),
            Some(PathBuf::from("/tmp/captura.png"))
        );
        assert_eq!(
            uri_to_path("file:///tmp/with%20space.png"),
            Some(PathBuf::from("/tmp/with space.png"))
        );
        assert_eq!(uri_to_path("https://example.com/a.png"), None);
    }
}
