//! **IPC** — adaptador de entrada (comandos e eventos Tauri).
//!
//! Apenas traduz transporte ↔ caso de uso. Não contém regra de negócio.

pub mod ai_source_commands;
pub mod ask_commands;
pub mod audio_commands;
pub mod database_commands;
pub mod export_commands;
pub mod journal_commands;
pub mod llm_commands;
pub mod menu;
pub mod model_commands;
pub mod overlay;
pub mod panel_commands;
pub mod ping;
pub mod recording_commands;
pub mod screen_commands;
pub mod session_commands;
pub mod session_end_commands;
pub mod settings_commands;
pub mod shortcuts_commands;
pub mod suggestion_commands;
pub mod summary_commands;
pub mod window_commands;
pub mod window_focus;
pub mod window_visibility;
