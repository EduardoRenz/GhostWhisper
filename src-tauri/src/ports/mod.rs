//! **Portas** — contratos (traits) do núcleo.
//!
//! Regras: sem prefixo `I`; nome descreve a capacidade. Implementações
//! concretas ficam em `adapters` e são prefixadas pelo contexto/tecnologia
//! (ex.: `LocalTranscriptionsRepository`, `OpenAiChatBotProvider`).
//!
//! Catálogo completo em `prds/mvp/architecture/engineering-guidelines.md`.

pub mod audio_consumer;
pub mod audio_monitor;
pub mod audio_reference;
pub mod audio_source;
pub mod chat_bot_provider;
pub mod click_through;
pub mod clock;
pub mod dwell_popover;
pub mod event_publisher;
pub mod gpu_probe;
pub mod llm_controller;
pub mod llm_provider_factory;
pub mod meeting_summaries_repository;
pub mod model_store;
pub mod overlay_window;
pub mod pointer_source;
pub mod repositories;
pub mod secret_store;
pub mod session_controller;
pub mod settings_store;
pub mod shortcut_registry;
pub mod speech_engine_factory;
pub mod speech_to_text;
pub mod suggestion_generator;
pub mod summary_generator;
pub mod translator;
pub mod virtual_sink;
