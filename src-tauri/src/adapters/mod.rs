//! **Adaptadores** — implementações concretas das portas.
//!
//! Subpastas: `audio/`, `speech/`, `ai/`, `persistence/`, `secrets/`,
//! `settings/`, `overlay/`, `shortcuts/`, `events/`, `system/`.
//! Inclui dobras de teste (`persistence/memory/`).
//!
//! Adaptadores só são instanciados no composition root (`lib.rs`).

pub mod ai;
pub mod audio;
pub mod dwell;
pub mod events;
pub mod manual;
pub mod overlay;
pub mod persistence;
pub mod secrets;
pub mod settings;
pub mod shortcuts;
pub mod speech;
pub mod system;
