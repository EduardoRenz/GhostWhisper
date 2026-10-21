//! Adaptadores da gravação da sessão (BR-065…BR-071).
//!
//! - `ffmpeg_path`: resolução do binário em degraus (RNF-09);
//! - `ffmpeg_args`: builders **puros** de argumentos (RNF-10);
//! - `ffmpeg_screen_recorder`: captura em segmentos MKV;
//! - `finalizer`: concat/remux e progresso;
//! - `x11_targets`: janela ativa (`_NET_ACTIVE_WINDOW`) e tela.

pub mod ffmpeg_args;
pub mod ffmpeg_path;
pub mod ffmpeg_screen_recorder;
pub mod finalizer;
pub mod x11_targets;
