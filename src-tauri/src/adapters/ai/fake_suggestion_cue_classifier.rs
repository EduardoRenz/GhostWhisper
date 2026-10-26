//! Dobra de teste: classificador determinístico, sem modelo/rede.

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::ports::suggestion_cue_classifier::{
    ClassificationOutcome, ClassificationRequest, ClassifierError, SuggestionCueClassifier,
};

type FakeResult = std::result::Result<ClassificationOutcome, ClassifierError>;

/// Classificador de teste: devolve resultados na ordem em que foram enfileirados
/// e registra cada request.
pub struct FakeSuggestionCueClassifier {
    results: Mutex<VecDeque<FakeResult>>,
    requests: Mutex<Vec<ClassificationRequest>>,
    backend: &'static str,
}

impl FakeSuggestionCueClassifier {
    /// Cria o fake vazio; sem resultados, `classify` devolve `Unavailable`.
    pub fn new() -> Self {
        Self {
            results: Mutex::new(VecDeque::new()),
            requests: Mutex::new(Vec::new()),
            backend: "fake",
        }
    }

    /// Define a identidade devolvida por `backend_id`.
    pub fn with_backend(mut self, backend: &'static str) -> Self {
        self.backend = backend;
        self
    }

    /// Enfileira um resultado de sucesso.
    pub fn push_outcome(&self, outcome: ClassificationOutcome) -> &Self {
        if let Ok(mut results) = self.results.lock() {
            results.push_back(Ok(outcome));
        }
        self
    }

    /// Enfileira um erro.
    pub fn push_error(&self, error: ClassifierError) -> &Self {
        if let Ok(mut results) = self.results.lock() {
            results.push_back(Err(error));
        }
        self
    }

    /// Requests recebidos, na ordem.
    pub fn requests(&self) -> Vec<ClassificationRequest> {
        self.requests
            .lock()
            .map(|requests| requests.clone())
            .unwrap_or_default()
    }
}

impl Default for FakeSuggestionCueClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl SuggestionCueClassifier for FakeSuggestionCueClassifier {
    fn classify(
        &self,
        request: &ClassificationRequest,
    ) -> std::result::Result<ClassificationOutcome, ClassifierError> {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(request.clone());
        }
        self.results
            .lock()
            .ok()
            .and_then(|mut results| results.pop_front())
            .unwrap_or(Err(ClassifierError::Unavailable))
    }

    fn backend_id(&self) -> &'static str {
        self.backend
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::suggestion_gate::{BlockTurn, ConversationBlock, SuggestionCue};
    use crate::domain::transcription::Speaker;

    fn block() -> ConversationBlock {
        ConversationBlock {
            id: "blk_1".to_string(),
            session_id: "s1".to_string(),
            start_ms: 0,
            end_ms: 1000,
            turns: vec![BlockTurn {
                speaker: Speaker::They,
                text: "pode explicar?".to_string(),
                start_ms: 0,
                end_ms: 1000,
            }],
            me_chars: 10,
            they_chars: 12,
            is_open: false,
        }
    }

    fn request() -> ClassificationRequest {
        ClassificationRequest {
            session_id: "s1".to_string(),
            block: block(),
            context: vec![],
            locale: "pt".to_string(),
            mode_id: "default".to_string(),
            mode_context: String::new(),
            allow_single_speaker: false,
            is_open_probe: false,
            now_ms: 1_000,
        }
    }

    fn outcome() -> ClassificationOutcome {
        ClassificationOutcome {
            block_id: "blk_1".to_string(),
            session_id: "s1".to_string(),
            cue: SuggestionCue::Question,
            should_suggest: true,
            confidence: 0.9,
            evidence: "pode explicar?".to_string(),
            focus: None,
            backend: "fake".to_string(),
            model: None,
            latency_ms: 1,
            block_start_ms: 0,
            block_end_ms: 1000,
            turn_count: 1,
            classified_at_ms: 1000,
            abstained: false,
        }
    }

    #[test]
    fn returns_queued_outcomes_and_errors_in_order() {
        let fake = FakeSuggestionCueClassifier::new();
        fake.push_outcome(outcome());
        fake.push_error(ClassifierError::Timeout);

        assert_eq!(fake.classify(&request()).expect("outcome"), outcome());
        assert!(matches!(
            fake.classify(&request()),
            Err(ClassifierError::Timeout)
        ));
    }

    #[test]
    fn records_each_request() {
        let fake = FakeSuggestionCueClassifier::new();
        fake.push_outcome(outcome());
        fake.push_outcome(outcome());

        let first = request();
        let mut second = request();
        second.is_open_probe = true;
        let _ = fake.classify(&first);
        let _ = fake.classify(&second);

        let requests = fake.requests();
        assert_eq!(requests.len(), 2);
        assert!(!requests[0].is_open_probe);
        assert!(requests[1].is_open_probe);
    }

    #[test]
    fn empty_queue_is_unavailable() {
        let fake = FakeSuggestionCueClassifier::new();
        assert!(matches!(
            fake.classify(&request()),
            Err(ClassifierError::Unavailable)
        ));
    }

    #[test]
    fn backend_id_is_configurable() {
        let fake = FakeSuggestionCueClassifier::new().with_backend("heuristic");
        assert_eq!(fake.backend_id(), "heuristic");
    }
}
