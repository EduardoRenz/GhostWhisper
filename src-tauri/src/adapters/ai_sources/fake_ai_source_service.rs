//! Dobra de teste: serviço de fontes com **roteiro** de respostas.
//!
//! Útil para verificar como a UI reage a uma fonte que falha, a uma chave
//! ausente ou a um informe de saúde específico.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::domain::ai_source::{AiSource, AiSourceHealth};
use crate::error::{GhostWhisperError, Result};
use crate::ports::ai_source_service::AiSourceService;

/// Resposta programada para a validação de uma fonte.
#[derive(Debug, Clone)]
pub enum FakeValidation {
    /// Responde com este texto.
    Ok(String),
    /// Falha com esta mensagem.
    Err(String),
}

impl Default for FakeValidation {
    fn default() -> Self {
        FakeValidation::Ok("ok".to_string())
    }
}

/// `AiSourceService` de teste com respostas programadas.
#[derive(Default)]
pub struct FakeAiSourceService {
    sources: Mutex<Vec<AiSource>>,
    keys: Mutex<HashMap<String, String>>,
    health: Mutex<Vec<AiSourceHealth>>,
    validations: Mutex<HashMap<String, FakeValidation>>,
    default_validation: Mutex<FakeValidation>,
}

impl FakeAiSourceService {
    /// Cria o fake (a validação padrão responde "ok").
    pub fn new() -> Self {
        let fake = Self::default();
        if let Ok(mut guard) = fake.default_validation.lock() {
            *guard = FakeValidation::Ok("ok".to_string());
        }
        fake
    }

    /// Programa a resposta da validação de uma fonte.
    pub fn with_validation(self, source_id: &str, validation: FakeValidation) -> Self {
        if let Ok(mut guard) = self.validations.lock() {
            guard.insert(source_id.to_string(), validation);
        }
        self
    }

    /// Programa a resposta padrão da validação.
    pub fn with_default_validation(self, validation: FakeValidation) -> Self {
        if let Ok(mut guard) = self.default_validation.lock() {
            *guard = validation;
        }
        self
    }

    /// Define a lista de fontes.
    pub fn with_sources(self, sources: Vec<AiSource>) -> Self {
        if let Ok(mut guard) = self.sources.lock() {
            *guard = sources;
        }
        self
    }

    /// Define a chave de uma fonte.
    pub fn with_key(self, source_id: &str, key: &str) -> Self {
        if let Ok(mut guard) = self.keys.lock() {
            guard.insert(source_id.to_string(), key.to_string());
        }
        self
    }

    /// Define o informe de saúde.
    pub fn with_health(self, health: Vec<AiSourceHealth>) -> Self {
        if let Ok(mut guard) = self.health.lock() {
            *guard = health;
        }
        self
    }
}

impl AiSourceService for FakeAiSourceService {
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
            .map(|value| crate::domain::settings::redact_secret(&value))
            .unwrap_or_default())
    }

    fn validate(&self, source: &AiSource) -> Result<String> {
        let programmed = self
            .validations
            .lock()
            .ok()
            .and_then(|guard| guard.get(&source.id).cloned());
        let validation = programmed.unwrap_or_else(|| {
            self.default_validation
                .lock()
                .map(|guard| guard.clone())
                .unwrap_or(FakeValidation::Ok("ok".to_string()))
        });
        match validation {
            FakeValidation::Ok(text) => Ok(text),
            FakeValidation::Err(message) => Err(GhostWhisperError::Other(message)),
        }
    }
}
