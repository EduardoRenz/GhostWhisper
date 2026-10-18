//! **Aplicação** — casos de uso.
//!
//! Orquestra o domínio dependendo apenas de `ports` (traits). Não faz I/O
//! direto: recebe as portas por injeção de dependência (construtor).

pub mod insight_service;
pub mod llm_router;
pub mod markdown_export;
pub mod prompts;
pub mod session_service;
pub mod session_timeline;
#[cfg(test)]
pub mod test_support;
pub mod transcription_pipeline;
pub mod translation_service;
pub mod turn_scheduler;
