//! Adaptadores da gravação da sessão (BR-065…BR-071).
//!
//! - `ffmpeg_path`: resolução do binário em degraus (RNF-09);
//! - `ffmpeg_args`: builders **puros** de argumentos (RNF-10);
//! - `ffmpeg_screen_recorder`: captura em segmentos MKV (X11);
//! - `gstreamer_args`: builder **puro** do pipeline `gst-launch` (Wayland);
//! - `portal_screen_recorder`: ScreenCast + PipeWire + GStreamer (Wayland);
//! - `portal_targets`: alvo único do seletor do GNOME (Wayland);
//! - `finalizer`: concat/remux e progresso;
//! - `x11_targets`: janela ativa (`_NET_ACTIVE_WINDOW`) e tela (X11).

pub mod ffmpeg_args;
pub mod ffmpeg_path;
pub mod ffmpeg_screen_recorder;
pub mod finalizer;
pub mod gstreamer_args;
pub mod portal_screen_recorder;
pub mod portal_targets;
pub mod x11_targets;
