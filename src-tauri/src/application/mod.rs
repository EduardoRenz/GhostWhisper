//! **Aplicação** — casos de uso.
//!
//! Orquestra o domínio dependendo apenas de `ports` (traits). Não faz I/O
//! direto: recebe as portas por injeção de dependência (construtor).

pub mod ai_source_health;
pub mod ai_source_migration;
pub mod ai_source_service;
pub mod answer_summary;
pub mod ask_service;
pub mod assistant_catalog;
pub mod assistant_catalog_migration;
pub mod assistant_mode_migration;
pub mod classifier_tick;
pub mod context;
pub mod llm_router;
pub mod llm_timeouts;
pub mod markdown_export;
pub mod prompts;
pub mod recording_service;
pub mod saved_questions_migration;
pub mod session_lookup;
pub mod session_service;
pub mod session_timeline;
pub mod suggestion_gate_service;
pub mod suggestion_scheduler;
pub mod suggestion_service;
pub mod summary_refresh;
pub mod summary_service;
#[cfg(test)]
pub mod test_support;
pub mod transcription_pipeline;
pub mod translation_service;
pub mod turn_scheduler;
pub mod ui_language_migration;
