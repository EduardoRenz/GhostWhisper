//! Resumo de fim de sessão em memória (transitório por definição).

use std::sync::Mutex;

use crate::error::{GhostWhisperError, Result};
use crate::ports::pending_session_end::PendingSessionEnd;

/// Guarda o id da sessão do modal.
#[derive(Default)]
pub struct InMemoryPendingSessionEnd {
    session_id: Mutex<Option<String>>,
}

impl InMemoryPendingSessionEnd {
    /// Cria o armazenamento vazio.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Option<String>>> {
        self.session_id
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock do resumo envenenado".to_string()))
    }
}

impl PendingSessionEnd for InMemoryPendingSessionEnd {
    fn set(&self, session_id: &str) -> Result<()> {
        *self.lock()? = Some(session_id.to_string());
        Ok(())
    }

    fn get(&self) -> Result<Option<String>> {
        Ok(self.lock()?.clone())
    }

    fn clear(&self) -> Result<()> {
        *self.lock()? = None;
        Ok(())
    }
}
