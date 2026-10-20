//! Captura de tela no X11/XWayland (`x11rb`) — BR-055.
//!
//! O Perssua usa o `desktopCapturer` do Electron; aqui a captura é do **X11**
//! (`GetImage` na root window), alinhado a BR-007 (X11/XWayland primeiro).
//! A imagem é reescalada para **≤1280 px** no lado maior e recomprimida em
//! **JPEG q0.86** — o preset `balanced` do Perssua, que corta o custo de
//! tokens sem perder legibilidade.

use std::io::Cursor;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat};
use x11rb::rust_connection::RustConnection;

use crate::error::{GhostWhisperError, Result};
use crate::ports::screen_capture::{CapturedImage, ScreenCapture};

/// Lado maior máximo da imagem enviada (paridade com o `balanced` do
/// Perssua: 1280 px).
pub const MAX_DIMENSION: u32 = 1280;
/// Qualidade do JPEG (paridade com o `balanced` do Perssua: 0.86).
pub const JPEG_QUALITY: u8 = 86;

/// Captura a tela pelo X11.
pub struct X11ScreenCapture;

impl X11ScreenCapture {
    /// Cria o capturador.
    pub fn new() -> Self {
        Self
    }
}

impl Default for X11ScreenCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenCapture for X11ScreenCapture {
    fn capture(&self) -> Result<CapturedImage> {
        let (connection, screen_num) = RustConnection::connect(None).map_err(|error| {
            GhostWhisperError::Other(format!(
                "captura de tela indisponível neste ambiente — cole a imagem com Ctrl+V ({error})"
            ))
        })?;
        let screen = &connection.setup().roots[screen_num];
        let root = screen.root;

        let width = u32::from(screen.width_in_pixels);
        let height = u32::from(screen.height_in_pixels);
        if width == 0 || height == 0 {
            return Err(GhostWhisperError::Other(
                "a tela não tem dimensões — cole a imagem com Ctrl+V".to_string(),
            ));
        }

        // As máscaras de canal do visual (RGB vs BGR) estão no *setup* do
        // servidor — não é preciso pedir `GetVisualInfo`.
        let visual = screen
            .allowed_depths
            .iter()
            .flat_map(|depth| depth.visuals.iter())
            .find(|visual| visual.visual_id == screen.root_visual)
            .ok_or_else(|| GhostWhisperError::Other("visual da tela indisponível".to_string()))?;

        let reply = connection
            .get_image(
                ImageFormat::Z_PIXMAP,
                root,
                0,
                0,
                width
                    .try_into()
                    .map_err(|_| GhostWhisperError::Other("tela larga demais".to_string()))?,
                height
                    .try_into()
                    .map_err(|_| GhostWhisperError::Other("tela alta demais".to_string()))?,
                u32::MAX,
            )
            .map_err(|error| GhostWhisperError::Other(format!("captura da tela: {error}")))?
            .reply()
            .map_err(|error| GhostWhisperError::Other(format!("captura da tela: {error}")))?;

        let rgb = to_rgb(
            &reply.data,
            width,
            height,
            visual.red_mask,
            visual.green_mask,
            visual.blue_mask,
        );

        let (data, out_width, out_height) = encode_jpeg(&rgb, width, height)?;
        Ok(CapturedImage {
            mime: "image/jpeg".to_string(),
            data,
            width: out_width,
            height: out_height,
        })
    }
}

/// Converte os pixels crus do X11 (BGRX/RGBX) em RGB.
///
/// As máscaras do visual dizem onde estão os canais: `0xFF0000/0xFF00/0xFF`
/// é RGB, `0xFF/0xFF00/0xFF0000` é BGR (o usual no X11).
fn to_rgb(
    raw: &[u8],
    width: u32,
    height: u32,
    red_mask: u32,
    green_mask: u32,
    blue_mask: u32,
) -> Vec<u8> {
    let bytes_per_pixel = if raw.len() >= (width as usize) * (height as usize) * 4 {
        4
    } else {
        3
    };
    let red = Channel::new(red_mask);
    let green = Channel::new(green_mask);
    let blue = Channel::new(blue_mask);

    let mut out = Vec::with_capacity((width as usize) * (height as usize) * 3);
    for pixel in raw.chunks_exact(bytes_per_pixel) {
        let value = u32::from_le_bytes([pixel[0], pixel[1], pixel[2], 0]);
        out.push(red.extract(value));
        out.push(green.extract(value));
        out.push(blue.extract(value));
    }
    out
}

/// Posição e alcance de um canal de cor (a partir da máscara do visual).
struct Channel {
    shift: u32,
    max: u32,
}

impl Channel {
    fn new(mask: u32) -> Self {
        if mask == 0 {
            return Self { shift: 0, max: 255 };
        }
        let shift = mask.trailing_zeros();
        let max = mask >> shift;
        Self { shift, max }
    }

    /// Extrai o canal (0–255) de um pixel.
    fn extract(&self, pixel: u32) -> u8 {
        if self.max == 0 {
            return 0;
        }
        let value = (pixel >> self.shift) & self.max;
        ((value * 255) / self.max) as u8
    }
}

/// Reescala (≤1280 px) e codifica em JPEG.
fn encode_jpeg(rgb: &[u8], width: u32, height: u32) -> Result<(Vec<u8>, u32, u32)> {
    use image::imageops::FilterType;
    use image::{ImageEncoder, RgbImage};

    let source = RgbImage::from_raw(width, height, rgb.to_vec())
        .ok_or_else(|| GhostWhisperError::Other("pixels da tela inválidos".to_string()))?;

    // `resize` não preserva a proporção: calculamos o alvo antes.
    let resized = if width.max(height) > MAX_DIMENSION {
        let scale = MAX_DIMENSION as f32 / width.max(height) as f32;
        let target_width = ((width as f32 * scale).round() as u32).max(1);
        let target_height = ((height as f32 * scale).round() as u32).max(1);
        image::imageops::resize(&source, target_width, target_height, FilterType::Triangle)
    } else {
        source
    };

    let mut buffer = Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, JPEG_QUALITY);
    encoder
        .write_image(
            resized.as_raw(),
            resized.width(),
            resized.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| GhostWhisperError::Other(format!("compressão da imagem: {error}")))?;

    Ok((buffer.into_inner(), resized.width(), resized.height()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_an_rgb_channel_from_the_mask() {
        // 0xFF0000 = vermelho no byte alto (padrão BGR do X11).
        let red = Channel::new(0x00FF_0000);
        assert_eq!(red.extract(0x00FF_0000), 255);
        assert_eq!(red.extract(0x0000_0000), 0);
        assert_eq!(red.extract(0x0080_0000), 128);
    }

    #[test]
    fn extracts_a_bgr_channel_from_the_mask() {
        // 0x0000FF = vermelho no byte baixo (padrão RGB).
        let red = Channel::new(0x0000_00FF);
        assert_eq!(red.extract(0x0000_00FF), 255);
        assert_eq!(red.extract(0x0000_0000), 0);
    }

    #[test]
    fn converts_bgrx_pixels_to_rgb() {
        // Um pixel BGRX: B=0x10, G=0x20, R=0x30.
        let raw = [0x10u8, 0x20, 0x30, 0x00];
        let rgb = to_rgb(&raw, 1, 1, 0x00FF_0000, 0x0000_FF00, 0x0000_00FF);
        assert_eq!(rgb, vec![0x30, 0x20, 0x10]);
    }

    #[test]
    fn converts_rgbx_pixels_to_rgb() {
        // Um pixel RGBX: R=0x10, G=0x20, B=0x30.
        let raw = [0x10u8, 0x20, 0x30, 0x00];
        let rgb = to_rgb(&raw, 1, 1, 0x0000_00FF, 0x0000_FF00, 0x00FF_0000);
        assert_eq!(rgb, vec![0x10, 0x20, 0x30]);
    }

    #[test]
    fn encodes_a_jpeg_within_the_dimension_limit() {
        let (width, height) = (200u32, 100u32);
        let rgb = vec![128u8; (width * height * 3) as usize];
        let (data, out_width, out_height) = encode_jpeg(&rgb, width, height).expect("jpeg");
        assert!(!data.is_empty());
        assert_eq!(out_width, width);
        assert_eq!(out_height, height);
        // Cabeçalho JPEG (SOI).
        assert_eq!(&data[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn shrinks_a_big_image_to_the_limit() {
        let (width, height) = (2560u32, 1440u32);
        let rgb = vec![200u8; (width * height * 3) as usize];
        let (_data, out_width, out_height) = encode_jpeg(&rgb, width, height).expect("jpeg");
        assert_eq!(out_width, MAX_DIMENSION);
        assert_eq!(out_height, 720);
    }

    /// Teste manual: precisa de um display X11 (roda com `cargo test -- --ignored`).
    #[test]
    #[ignore = "precisa de um display X11"]
    fn captures_the_real_screen() {
        let image = X11ScreenCapture::new().capture().expect("captura");
        assert_eq!(image.mime, "image/jpeg");
        assert!(!image.data.is_empty());
        assert!(image.width > 0 && image.height > 0);
        assert!(image.width.max(image.height) <= MAX_DIMENSION);
        // Cabeçalho JPEG (SOI).
        assert_eq!(&image.data[..2], &[0xFF, 0xD8]);
    }
}
