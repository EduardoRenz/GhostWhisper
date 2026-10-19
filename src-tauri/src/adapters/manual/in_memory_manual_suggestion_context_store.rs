//! Store em memória do contexto manual (dobra de teste do port).

use std::sync::Mutex;

use crate::error::Result;
use crate::ports::manual_suggestion_context::{
    ManualSuggestionContext, ManualSuggestionContextStore,
};

/// Guarda no máximo um contexto manual por vez.
#[derive(Default)]
pub struct InMemoryManualSuggestionContextStore {
    context: Mutex<Option<ManualSuggestionContext>>,
}

impl InMemoryManualSuggestionContextStore {
    /// Cria um store vazio.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ManualSuggestionContextStore for InMemoryManualSuggestionContextStore {
    fn put(&self, context: ManualSuggestionContext) -> Result<()> {
        let mut slot = self
            .context
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = Some(context);
        Ok(())
    }

    fn get(&self, session_id: &str) -> Result<Option<ManualSuggestionContext>> {
        let slot = self
            .context
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(slot
            .as_ref()
            .filter(|context| context.session_id == session_id)
            .cloned())
    }

    fn clear(&self, session_id: &str) -> Result<()> {
        let mut slot = self
            .context
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot
            .as_ref()
            .map(|context| context.session_id == session_id)
            .unwrap_or(false)
        {
            *slot = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::transcription::Speaker;

    fn context(session_id: &str) -> ManualSuggestionContext {
        ManualSuggestionContext {
            session_id: session_id.to_string(),
            speaker: Speaker::Me,
            text: "ok, vamos começar".to_string(),
            at_ms: 1_000,
        }
    }

    #[test]
    fn stores_replaces_and_clears_the_context() {
        let store = InMemoryManualSuggestionContextStore::new();
        store.put(context("s1")).expect("put");
        assert_eq!(
            store.get("s1").expect("get").map(|entry| entry.text),
            Some("ok, vamos começar".to_string())
        );

        store.put(context("s2")).expect("substitui");
        assert!(store.get("s1").expect("get s1").is_none());
        assert!(store.get("s2").expect("get s2").is_some());

        store.clear("s1").expect("clear de outra sessão");
        assert!(store.get("s2").expect("s2 continua").is_some());
        store.clear("s2").expect("clear");
        assert!(store.get("s2").expect("get").is_none());
    }
}
