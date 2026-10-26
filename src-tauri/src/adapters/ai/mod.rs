//! Adaptadores de LLM (providers BYO key).

pub mod custom_chat_bot_provider;
pub mod deepseek_chat_bot_provider;
pub mod default_chat_bot_provider_factory;
#[cfg(test)]
pub mod fake_chat_bot_provider;
#[cfg(test)]
pub mod fake_suggestion_cue_classifier;
pub mod gemini_chat_bot_provider;
pub mod heuristic_suggestion_cue_classifier;
// O adaptador Laya compila sem o engine: o binding ONNX entra quando o
// `laya-onnx`/artefato forem publicados (nota na task 06).
#[cfg(all(test, feature = "laya"))]
pub mod laya_integration;
#[cfg(feature = "laya")]
pub mod laya_onnx_predictor;
pub mod laya_suggestion_cue_classifier;
pub mod openai_chat_bot_provider;
pub mod openai_compatible;
pub mod openrouter_chat_bot_provider;
pub mod settings_suggestion_cue_classifier;
