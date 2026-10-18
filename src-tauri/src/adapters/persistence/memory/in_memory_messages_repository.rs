//! Dobra de teste para `MessagesRepository`.

use std::sync::Mutex;

use crate::domain::message::ConversationMessage;
use crate::error::{GhostWhisperError, Result};
use crate::ports::repositories::MessagesRepository;

/// Repositório de mensagens em memória.
#[derive(Default)]
pub struct InMemoryMessagesRepository {
    messages: Mutex<Vec<ConversationMessage>>,
}

impl InMemoryMessagesRepository {
    /// Cria um repositório vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retorna uma cópia de todas as mensagens.
    pub fn all(&self) -> Vec<ConversationMessage> {
        self.messages.lock().map(|m| m.clone()).unwrap_or_default()
    }
}

impl MessagesRepository for InMemoryMessagesRepository {
    fn insert(&self, message: &ConversationMessage) -> Result<()> {
        self.messages
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de mensagens envenenado".to_string()))?
            .push(message.clone());
        Ok(())
    }

    fn list_by_session(&self, session_id: &str) -> Result<Vec<ConversationMessage>> {
        let mut messages: Vec<_> = self
            .all()
            .into_iter()
            .filter(|message| message.session_id == session_id)
            .collect();
        messages.sort_by_key(|message| message.timestamp_ms);
        Ok(messages)
    }
}
