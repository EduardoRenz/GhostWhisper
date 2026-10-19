//! **IPC** — adaptador de entrada (comandos e eventos Tauri).
//!
//! Apenas traduz transporte ↔ caso de uso. Não contém regra de negócio.

pub mod audio_commands;
pub mod database_commands;
pub mod dwell;
pub mod export_commands;
pub mod llm_commands;
pub mod overlay;
pub mod panel_commands;
pub mod ping;
pub mod session_commands;
pub mod settings_commands;
pub mod shortcuts_commands;
pub mod suggestion_commands;
pub mod summary_commands;
