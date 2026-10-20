//! Dobra de teste para `JournalRepository`.

use std::sync::Mutex;

use crate::domain::journal::{JournalEntry, JournalImage, JournalRequestSource, JournalStatus};
use crate::error::{GhostWhisperError, Result};
use crate::ports::journal_repository::JournalRepository;

/// Diário de perguntas em memória.
#[derive(Default)]
pub struct InMemoryJournalRepository {
    entries: Mutex<Vec<JournalEntry>>,
    images: Mutex<Vec<JournalImage>>,
}

impl InMemoryJournalRepository {
    /// Cria um repositório vazio.
    pub fn new() -> Self {
        Self::default()
    }

    fn locked<'a, T>(lock: &'a Mutex<T>, what: &str) -> Result<std::sync::MutexGuard<'a, T>> {
        lock.lock()
            .map_err(|_| GhostWhisperError::Other(format!("lock de {what} envenenado")))
    }
}

impl JournalRepository for InMemoryJournalRepository {
    fn insert_question(&self, entry: &JournalEntry) -> Result<()> {
        Self::locked(&self.entries, "diário")?.push(entry.clone());
        Ok(())
    }

    fn complete_answer(
        &self,
        id: &str,
        answer: &str,
        answered_at_ms: i64,
        provider: Option<String>,
        model: Option<String>,
    ) -> Result<()> {
        if let Ok(mut entries) = self.entries.lock() {
            if let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) {
                entry.answer = Some(answer.to_string());
                entry.answered_at_ms = Some(answered_at_ms);
                entry.status = JournalStatus::Answered;
                if provider.is_some() {
                    entry.provider = provider;
                }
                if model.is_some() {
                    entry.model = model;
                }
            }
        }
        Ok(())
    }

    fn fail(&self, id: &str, status: JournalStatus, error: Option<String>) -> Result<()> {
        if let Ok(mut entries) = self.entries.lock() {
            if let Some(entry) = entries.iter_mut().find(|entry| entry.id == id) {
                entry.status = status;
                entry.error = error;
            }
        }
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<JournalEntry>> {
        let entries = Self::locked(&self.entries, "diário")?;
        Ok(entries.iter().find(|entry| entry.id == id).cloned())
    }

    fn list_by_session(&self, session_id: &str) -> Result<Vec<JournalEntry>> {
        let entries = Self::locked(&self.entries, "diário")?;
        let mut listed: Vec<JournalEntry> = entries
            .iter()
            .filter(|entry| entry.session_id == session_id)
            .cloned()
            .collect();
        listed.sort_by(|left, right| {
            left.asked_at_ms
                .cmp(&right.asked_at_ms)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(listed)
    }

    fn search(&self, session_id: &str, query: &str, limit: usize) -> Result<Vec<JournalEntry>> {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|term| term.to_lowercase())
            .collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let listed = self.list_by_session(session_id)?;
        let mut found: Vec<JournalEntry> = listed
            .into_iter()
            .filter(|entry| {
                let haystack = format!(
                    "{} {}",
                    entry.question.to_lowercase(),
                    entry.answer.as_deref().unwrap_or_default().to_lowercase()
                );
                terms.iter().all(|term| haystack.contains(term))
            })
            .collect();
        found.truncate(limit);
        Ok(found)
    }

    fn attach_image(&self, image: &JournalImage) -> Result<()> {
        Self::locked(&self.images, "imagens")?.push(image.clone());
        Ok(())
    }

    fn images_of(&self, entry_id: &str) -> Result<Vec<JournalImage>> {
        let images = Self::locked(&self.images, "imagens")?;
        Ok(images
            .iter()
            .filter(|image| image.entry_id == entry_id)
            .cloned()
            .collect())
    }

    fn delete(&self, id: &str) -> Result<()> {
        if let Ok(mut entries) = self.entries.lock() {
            entries.retain(|entry| entry.id != id);
        }
        if let Ok(mut images) = self.images.lock() {
            images.retain(|image| image.entry_id != id);
        }
        Ok(())
    }
}

/// Entrada de teste (atalho para as dobras).
pub fn entry(id: &str, session_id: &str, question: &str, at: i64) -> JournalEntry {
    JournalEntry {
        id: id.to_string(),
        session_id: session_id.to_string(),
        question: question.to_string(),
        answer: None,
        asked_at_ms: at,
        answered_at_ms: None,
        created_at_ms: at,
        status: JournalStatus::Pending,
        request_source: JournalRequestSource::Search,
        provider: None,
        model: None,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_answers_and_images() {
        let repo = InMemoryJournalRepository::new();
        repo.insert_question(&entry("e1", "s1", "pergunta", 0))
            .unwrap();
        repo.complete_answer("e1", "resposta", 10, None, None)
            .unwrap();

        let stored = repo.get("e1").unwrap().expect("presente");
        assert_eq!(stored.answer.as_deref(), Some("resposta"));
        assert_eq!(stored.status, JournalStatus::Answered);

        repo.attach_image(&JournalImage {
            id: "i1".into(),
            entry_id: "e1".into(),
            mime: "image/jpeg".into(),
            data: vec![1],
            width: None,
            height: None,
        })
        .unwrap();
        assert_eq!(repo.images_of("e1").unwrap().len(), 1);

        repo.delete("e1").unwrap();
        assert!(repo.get("e1").unwrap().is_none());
        assert!(repo.images_of("e1").unwrap().is_empty());
    }

    #[test]
    fn searches_inside_the_session_only() {
        let repo = InMemoryJournalRepository::new();
        repo.insert_question(&entry("e1", "s1", "linked list", 0))
            .unwrap();
        repo.insert_question(&entry("e2", "s2", "linked list", 0))
            .unwrap();

        let found = repo.search("s1", "linked", 10).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "e1");
    }
}
