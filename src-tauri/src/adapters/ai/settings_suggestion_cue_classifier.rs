//! Seletor de backend do classificador (BR-098).
//!
//! O gate recebe **um** classificador no composition root, mas o usuário troca
//! o backend nas configurações sem reiniciar o app. Este decorador lê a
//! configuração a cada bloco e delega:
//!
//! - `heuristic` → regras locais;
//! - `laya` com engine disponível → Laya;
//! - `laya` sem engine (build sem `laya`/modelo ausente) → heurístico, com um
//!   único aviso no log. A UI mostra o backend efetivo no `outcome.backend`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::domain::suggestion_gate::SuggestionCue;
use crate::ports::settings_store::SettingsStore;
use crate::ports::suggestion_cue_classifier::{
    ClassificationOutcome, ClassificationRequest, ClassifierError, SuggestionCueClassifier,
};

/// Backend efetivo escolhido pelas configurações.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifierBackend {
    Heuristic,
    Laya,
}

impl ClassifierBackend {
    pub fn from_settings(value: &str) -> Self {
        if value == "laya" {
            Self::Laya
        } else {
            Self::Heuristic
        }
    }
}

/// Classificador que respeita `suggestion_classifier_backend` em runtime.
pub struct SettingsSuggestionCueClassifier {
    settings: Arc<dyn SettingsStore>,
    heuristic: Arc<dyn SuggestionCueClassifier>,
    laya: Option<Arc<dyn SuggestionCueClassifier>>,
    warned: AtomicBool,
}

impl SettingsSuggestionCueClassifier {
    pub fn new(
        settings: Arc<dyn SettingsStore>,
        heuristic: Arc<dyn SuggestionCueClassifier>,
        laya: Option<Arc<dyn SuggestionCueClassifier>>,
    ) -> Self {
        Self {
            settings,
            heuristic,
            laya,
            warned: AtomicBool::new(false),
        }
    }

    fn select(&self) -> (ClassifierBackend, &dyn SuggestionCueClassifier) {
        let backend = self
            .settings
            .load()
            .map(|settings| {
                ClassifierBackend::from_settings(&settings.suggestion_classifier_backend)
            })
            .unwrap_or(ClassifierBackend::Heuristic);
        match backend {
            ClassifierBackend::Laya => match self.laya.as_deref() {
                Some(laya) => (ClassifierBackend::Laya, laya),
                None => {
                    if !self.warned.swap(true, Ordering::Relaxed) {
                        tracing::warn!(
                            "backend Laya indisponível neste build/modelo; usando regras locais"
                        );
                    }
                    (ClassifierBackend::Heuristic, self.heuristic.as_ref())
                }
            },
            ClassifierBackend::Heuristic => (ClassifierBackend::Heuristic, self.heuristic.as_ref()),
        }
    }
}

impl SuggestionCueClassifier for SettingsSuggestionCueClassifier {
    fn classify(
        &self,
        request: &ClassificationRequest,
    ) -> std::result::Result<ClassificationOutcome, ClassifierError> {
        let (backend, classifier) = self.select();
        let outcome = classifier.classify(request)?;
        if backend != ClassifierBackend::Laya {
            return Ok(outcome);
        }

        // O Laya multilingual é zero-shot e não calibrado. As regras locais
        // entram como guarda determinística:
        // - pergunta repetida: nunca sugerir de novo (`already_answered`);
        // - Laya abstém ou diz `no_action`: se as regras veem pergunta/convite,
        //   usar as regras (recall);
        // - Laya positivo: preservar a decisão contextual do modelo.
        let fallback = self.heuristic.classify(request)?;
        if fallback.cue == SuggestionCue::AlreadyAnswered {
            return Ok(fallback);
        }
        // Sem evidência determinística de repetição, o Laya não pode “inventar”
        // already_answered (isso suprime uma pergunta válida).
        if outcome.cue == SuggestionCue::AlreadyAnswered {
            return Ok(fallback);
        }
        // Comentário só vale quando as regras também viram um convite
        // explícito. Uma opinião/afirmação de THEY não é oportunidade.
        if outcome.cue == SuggestionCue::Comment && fallback.cue != SuggestionCue::Comment {
            return Ok(fallback);
        }
        // Convite de comentário detectado pelas regras vence uma pergunta
        // retórica do Laya (ambos são positivos; o foco muda).
        if fallback.cue == SuggestionCue::Comment && outcome.cue == SuggestionCue::Question {
            return Ok(fallback);
        }
        if matches!(
            outcome.cue,
            SuggestionCue::InsufficientContext | SuggestionCue::NoAction
        ) && fallback.should_suggest
        {
            return Ok(fallback);
        }
        if outcome.cue == SuggestionCue::InsufficientContext {
            return Ok(fallback);
        }
        // Bloco curto é abstenção pelas regras, mesmo que o Laya tente decidir.
        if outcome.cue == SuggestionCue::NoAction
            && fallback.cue == SuggestionCue::InsufficientContext
        {
            return Ok(fallback);
        }
        Ok(outcome)
    }

    fn backend_id(&self) -> &'static str {
        "settings"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::adapters::ai::fake_suggestion_cue_classifier::FakeSuggestionCueClassifier;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::domain::settings::Settings;
    use crate::domain::suggestion_gate::{BlockTurn, ConversationBlock, SuggestionCue};
    use crate::domain::transcription::Speaker;

    fn request() -> ClassificationRequest {
        let block = ConversationBlock {
            id: "blk_1".to_string(),
            session_id: "s1".to_string(),
            start_ms: 0,
            end_ms: 1_000,
            turns: vec![BlockTurn {
                speaker: Speaker::They,
                text: "pode explicar?".to_string(),
                start_ms: 0,
                end_ms: 1_000,
            }],
            me_chars: 10,
            they_chars: 12,
            is_open: false,
        };
        ClassificationRequest {
            session_id: "s1".to_string(),
            block: block.clone(),
            full_block: block,
            context: vec![],
            locale: "pt".to_string(),
            mode_id: "default".to_string(),
            mode_context: String::new(),
            allow_single_speaker: false,
            prior_questions: vec![],
            now_ms: 1_000,
        }
    }

    fn outcome(backend: &str) -> ClassificationOutcome {
        ClassificationOutcome {
            block_id: "blk_1".to_string(),
            session_id: "s1".to_string(),
            cue: SuggestionCue::Question,
            should_suggest: true,
            confidence: 0.9,
            evidence: "pode explicar?".to_string(),
            focus: None,
            backend: backend.to_string(),
            model: None,
            latency_ms: 0,
            block_start_ms: 0,
            block_end_ms: 1_000,
            turn_count: 1,
            classified_at_ms: 1_000,
            abstained: false,
        }
    }

    fn store(backend: &str) -> Arc<InMemorySettingsStore> {
        Arc::new(InMemorySettingsStore::with_settings(Settings {
            suggestion_classifier_backend: backend.to_string(),
            ..Settings::default()
        }))
    }

    #[test]
    fn uses_the_heuristic_when_selected() {
        let heuristic = Arc::new(FakeSuggestionCueClassifier::new());
        heuristic.push_outcome(outcome("heuristic"));
        let classifier = SettingsSuggestionCueClassifier::new(
            store("heuristic"),
            heuristic.clone(),
            Some(Arc::new(FakeSuggestionCueClassifier::new())),
        );

        let result = classifier.classify(&request()).expect("heurístico");
        assert_eq!(result.backend, "heuristic");
        assert_eq!(heuristic.requests().len(), 1);
    }

    #[test]
    fn uses_laya_when_selected_and_available() {
        let heuristic = Arc::new(FakeSuggestionCueClassifier::new());
        heuristic.push_outcome(outcome("heuristic"));
        let laya = Arc::new(FakeSuggestionCueClassifier::new());
        laya.push_outcome(outcome("laya"));
        let classifier = SettingsSuggestionCueClassifier::new(
            store("laya"),
            heuristic.clone(),
            Some(laya.clone()),
        );

        let result = classifier.classify(&request()).expect("laya");
        assert_eq!(result.backend, "laya");
        assert_eq!(heuristic.requests().len(), 1);
        assert_eq!(laya.requests().len(), 1);
    }

    #[test]
    fn falls_back_to_the_heuristic_when_laya_is_unavailable() {
        let heuristic = Arc::new(FakeSuggestionCueClassifier::new());
        heuristic.push_outcome(outcome("heuristic"));
        let classifier =
            SettingsSuggestionCueClassifier::new(store("laya"), heuristic.clone(), None);

        let result = classifier.classify(&request()).expect("fallback");
        assert_eq!(result.backend, "heuristic");
        assert_eq!(heuristic.requests().len(), 1);
    }

    #[test]
    fn falls_back_to_heuristic_when_laya_abstains() {
        let heuristic = Arc::new(FakeSuggestionCueClassifier::new());
        let mut heuristic_outcome = outcome("heuristic");
        heuristic_outcome.cue = SuggestionCue::Comment;
        heuristic_outcome.should_suggest = true;
        heuristic.push_outcome(heuristic_outcome);

        let laya = Arc::new(FakeSuggestionCueClassifier::new());
        let mut abstained = outcome("laya");
        abstained.cue = SuggestionCue::InsufficientContext;
        abstained.should_suggest = false;
        abstained.abstained = true;
        laya.push_outcome(abstained);

        let classifier = SettingsSuggestionCueClassifier::new(
            store("laya"),
            heuristic.clone(),
            Some(laya.clone()),
        );
        let result = classifier.classify(&request()).expect("fallback");
        assert_eq!(result.backend, "heuristic");
        assert_eq!(result.cue, SuggestionCue::Comment);
        assert!(result.should_suggest);
    }

    #[test]
    fn repeated_question_overrides_a_positive_laya_decision() {
        use crate::domain::suggestion_gate::PriorQuestion;

        let heuristic = Arc::new(FakeSuggestionCueClassifier::new());
        let mut answered = outcome("heuristic");
        answered.cue = SuggestionCue::AlreadyAnswered;
        answered.should_suggest = false;
        heuristic.push_outcome(answered);

        let laya = Arc::new(FakeSuggestionCueClassifier::new());
        laya.push_outcome(outcome("laya"));

        let classifier = SettingsSuggestionCueClassifier::new(store("laya"), heuristic, Some(laya));
        let mut request = request();
        request.prior_questions = vec![PriorQuestion {
            title: "pode explicar?".to_string(),
            block_id: "blk_old".to_string(),
            answered_at_ms: 1,
        }];
        let result = classifier.classify(&request).expect("guarda");
        assert_eq!(result.cue, SuggestionCue::AlreadyAnswered);
        assert!(!result.should_suggest);
    }

    #[test]
    fn backend_id_is_neutral() {
        let classifier = SettingsSuggestionCueClassifier::new(
            store("heuristic"),
            Arc::new(FakeSuggestionCueClassifier::new()),
            None,
        );
        assert_eq!(classifier.backend_id(), "settings");
    }
}
