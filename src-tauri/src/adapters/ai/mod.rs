//! Adaptadores de LLM (providers BYO key).

pub mod custom_chat_bot_provider;
pub mod deepseek_chat_bot_provider;
pub mod default_chat_bot_provider_factory;
#[cfg(test)]
pub mod fake_chat_bot_provider;
pub mod gemini_chat_bot_provider;
pub mod openai_chat_bot_provider;
pub mod openai_compatible;
pub mod openrouter_chat_bot_provider;
