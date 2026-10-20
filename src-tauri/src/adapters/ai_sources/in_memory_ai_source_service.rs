//! Dobra de teste: serviço de fontes em memória.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::domain::ai_source::{AiSource, AiSourceHealth};
use crate::domain::settings::redact_secret;
use crate::error::{GhostWhisperError, Result};
use crate::ports::ai_source_service::AiSourceService;

/// `AiSourceService` em memória (testes).
#[derive(Default)]
pub struct InMemoryAiSourceService {
    sources: Mutex<Vec<AiSource>>,
    keys: Mutex<HashMap<String, String>>,
    health: Mutex<Vec<AiSourceHealth>>,
}

impl InMemoryAiSourceService {
    /// Cria o serviço vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cria o serviço com uma lista de fontes.
    pub fn with_sources(sources: Vec<AiSource>) -> Self {
        Self {
            sources: Mutex::new(sources),
            keys: Mutex::new(HashMap::new()),
            health: Mutex::new(Vec::new()),
        }
    }

    /// Semeia uma chave para uma fonte.
    pub fn with_key(self, source_id: &str, key: &str) -> Self {
        if let Ok(mut guard) = self.keys.lock() {
            guard.insert(source_id.to_string(), key.to_string());
        }
        self
    }

    /// Semeia um informe de saúde.
    pub fn with_health(self, health: Vec<AiSourceHealth>) -> Self {
        if let Ok(mut guard) = self.health.lock() {
            *guard = health;
        }
        self
    }
}

impl AiSourceService for InMemoryAiSourceService {
    fn list(&self) -> Result<Vec<AiSource>> {
        Ok(self
            .sources
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default())
    }

    fn health(&self) -> Vec<AiSourceHealth> {
        self.health
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    fn set_key(&self, source_id: &str, key: &str) -> Result<()> {
        if let Ok(mut guard) = self.keys.lock() {
            guard.insert(source_id.to_string(), key.to_string());
        }
        Ok(())
    }

    fn has_key(&self, source_id: &str) -> Result<bool> {
        Ok(self
            .keys
            .lock()
            .map(|guard| guard.contains_key(source_id))
            .unwrap_or(false))
    }

    fn clear_key(&self, source_id: &str) -> Result<()> {
        if let Ok(mut guard) = self.keys.lock() {
            guard.remove(source_id);
        }
        Ok(())
    }

    fn key_preview(&self, source_id: &str) -> Result<String> {
        Ok(self
            .keys
            .lock()
            .ok()
            .and_then(|guard| guard.get(source_id).cloned())
            .map(|value| redact_secret(&value))
            .unwrap_or_default())
    }

    fn validate(&self, source: &AiSource) -> Result<String> {
        let has_key = self.has_key(&source.id)?;
        if !has_key {
            return Err(GhostWhisperError::Config(format!(
                "nenhuma chave configurada para {}",
                source.display_label()
            )));
        }
        Ok("ok".to_string())
    }
}
