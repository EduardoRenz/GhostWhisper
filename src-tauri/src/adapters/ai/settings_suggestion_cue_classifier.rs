//! Seletor de backend do classificador (BR-098).
//!
//! O gate recebe **um** classificador no composition root, mas o usuário troca
//! o backend nas configurações sem reiniciar o app. Este decorador lê a
//! configuração a cada bloco e delega:
//!
//! - `laya` (recomendado/efetivo) → Laya; a guarda **lexical** (Jaccard) de
//!   `already_answered` roda antes, independente de idioma (R23);
//! - `laya` sem engine (build sem `laya`/modelo ausente) → `Unavailable` (não
//!   cai em palavras-chave);
//! - `heuristic` → backend legado “Regras locais”, em remoção (ISS-078/task 16).
//!   A UI mostra o backend efetivo no `outcome.backend`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

    fn selected_backend(&self) -> ClassifierBackend {
        let value = self
            .settings
            .load()
            .map(|settings| settings.suggestion_classifier_backend)
            .unwrap_or_else(|_| "heuristic".to_string());
        ClassifierBackend::from_settings(&value)
    }
}

impl SuggestionCueClassifier for SettingsSuggestionCueClassifier {
    fn classify(
        &self,
        request: &ClassificationRequest,
    ) -> std::result::Result<ClassificationOutcome, ClassifierError> {
        let backend = self.selected_backend();
        if backend == ClassifierBackend::Heuristic {
            return self.heuristic.classify(request);
        }

        let Some(laya) = self.laya.as_deref() else {
            if !self.warned.swap(true, Ordering::Relaxed) {
                tracing::warn!(
                    "backend Laya indisponível neste build/modelo; nenhuma sugestão será gerada"
                );
            }
            return Err(ClassifierError::Unavailable);
        };

        // Guarda **independente de idioma** para pergunta repetida: similaridade
        // lexical (Jaccard) contra o histórico. O modelo zero-shot não é
        // confiável em `already_answered`; o resto da decisão é do Laya (R23).
        if let Some(outcome) = self.heuristic.already_answered(request) {
            return Ok(outcome);
        }
        laya.classify(request)
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
        // R23: o heurístico não participa da decisão positiva do Laya.
        assert_eq!(heuristic.requests().len(), 0);
        assert_eq!(laya.requests().len(), 1);
    }

    #[test]
    fn errors_when_laya_is_unavailable_instead_of_using_keywords() {
        let heuristic = Arc::new(FakeSuggestionCueClassifier::new());
        heuristic.push_outcome(outcome("heuristic"));
        let classifier =
            SettingsSuggestionCueClassifier::new(store("laya"), heuristic.clone(), None);

        assert!(classifier.classify(&request()).is_err());
        assert_eq!(heuristic.requests().len(), 0);
    }

    #[test]
    fn does_not_override_laya_abstention_with_keywords() {
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
        let result = classifier.classify(&request()).expect("laya");
        assert_eq!(result.backend, "laya");
        assert_eq!(result.cue, SuggestionCue::InsufficientContext);
        assert!(!result.should_suggest);
        assert_eq!(heuristic.requests().len(), 0);
    }

    #[test]
    fn repeated_question_overrides_a_positive_laya_decision() {
        use crate::adapters::ai::heuristic_suggestion_cue_classifier::HeuristicSuggestionCueClassifier;
        use crate::domain::suggestion_gate::PriorQuestion;

        let laya = Arc::new(FakeSuggestionCueClassifier::new());
        laya.push_outcome(outcome("laya"));

        let classifier = SettingsSuggestionCueClassifier::new(
            store("laya"),
            Arc::new(HeuristicSuggestionCueClassifier::default()),
            Some(laya),
        );
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
    fn a_long_repeated_question_with_a_different_start_overrides_laya() {
        use crate::adapters::ai::heuristic_suggestion_cue_classifier::HeuristicSuggestionCueClassifier;
        use crate::domain::suggestion_gate::PriorQuestion;

        let laya = Arc::new(FakeSuggestionCueClassifier::new());
        laya.push_outcome(outcome("laya"));
        let classifier = SettingsSuggestionCueClassifier::new(
            store("laya"),
            Arc::new(HeuristicSuggestionCueClassifier::default()),
            Some(laya.clone()),
        );

        // Sessão `5f8595a7…`: a repetição começa no meio da pergunta e o
        // histórico guardou só a segunda metade (ISS-081).
        let second_half = "você colocaria o load balancer na arquitetura e quais ferramentas ou tecnologias utilizaria para implementá-lo. Explique também a diferença entre usar um load balancer gerenciado na nuvem, como AWS LB, e uma solução própria, como Enginix ou HAPROXY.";
        let repeated = "web que receberá milhões de requisições por dia e terá várias instâncias do back-end rodando em paralelo, onde você colocaria o load balancer na arquitetura e quais ferramentas ou tecnologias utilizaria para implementá-lo. Explique também a diferença entre usar um load balancer gerenciado na nuvem, como AWS ALB, e uma solução própria, como Enginix ou Haproxy.";
        let mut request = request();
        request.block.turns = vec![BlockTurn {
            speaker: Speaker::They,
            text: repeated.to_string(),
            start_ms: 0,
            end_ms: 40_000,
        }];
        request.block.end_ms = 40_000;
        request.block.they_chars = repeated.chars().count();
        request.full_block = request.block.clone();
        request.prior_questions = vec![PriorQuestion {
            title: second_half.chars().take(180).collect(),
            block_id: "blk_old".to_string(),
            answered_at_ms: 1,
        }];

        let result = classifier.classify(&request).expect("guarda");
        assert_eq!(result.cue, SuggestionCue::AlreadyAnswered);
        assert_eq!(result.backend, "heuristic");
        assert!(
            laya.requests().is_empty(),
            "a guarda não deve chamar o Laya"
        );
    }

    #[test]
    fn the_tail_of_a_repeated_question_overrides_laya() {
        use crate::adapters::ai::heuristic_suggestion_cue_classifier::HeuristicSuggestionCueClassifier;
        use crate::domain::suggestion_gate::PriorQuestion;

        let laya = Arc::new(FakeSuggestionCueClassifier::new());
        laya.push_outcome(outcome("laya"));
        let classifier = SettingsSuggestionCueClassifier::new(
            store("laya"),
            Arc::new(HeuristicSuggestionCueClassifier::default()),
            Some(laya.clone()),
        );

        // Sessão `5f8595a7…`: a pergunta do amigo/dinheiro voltou só como
        // cauda; a contenção lexical reconhece o fragmento (ISS-081).
        let full = "Seguinte, um amigo te pede dinheiro emprestado toda semana e nunca devolve. O que você faz? A, continue emprestando porque é seu amigo. B, para de emprestar e conversa com ele com sinceridade. C, ignora e muda de assunto.";
        let tail = "Estar e conversa com ele com sinceridade. C. Ignora e muda de assunto.";
        let mut request = request();
        request.block.turns = vec![BlockTurn {
            speaker: Speaker::They,
            text: tail.to_string(),
            start_ms: 0,
            end_ms: 20_000,
        }];
        request.block.end_ms = 20_000;
        request.block.they_chars = tail.chars().count();
        request.full_block = request.block.clone();
        request.prior_questions = vec![PriorQuestion {
            title: full.to_string(),
            block_id: "blk_old".to_string(),
            answered_at_ms: 1,
        }];

        let result = classifier.classify(&request).expect("guarda");
        assert_eq!(result.cue, SuggestionCue::AlreadyAnswered);
        assert_eq!(result.backend, "heuristic");
        assert!(
            laya.requests().is_empty(),
            "a guarda não deve chamar o Laya"
        );
    }

    #[test]
    fn an_unrelated_question_still_reaches_laya() {
        use crate::adapters::ai::heuristic_suggestion_cue_classifier::HeuristicSuggestionCueClassifier;
        use crate::domain::suggestion_gate::PriorQuestion;

        let laya = Arc::new(FakeSuggestionCueClassifier::new());
        laya.push_outcome(outcome("laya"));
        let classifier = SettingsSuggestionCueClassifier::new(
            store("laya"),
            Arc::new(HeuristicSuggestionCueClassifier::default()),
            Some(laya.clone()),
        );
        let mut request = request();
        request.block.turns = vec![BlockTurn {
            speaker: Speaker::They,
            text: "Como você projetaria um sistema de filas com garantia de entrega?".to_string(),
            start_ms: 0,
            end_ms: 5_000,
        }];
        request.block.end_ms = 5_000;
        request.full_block = request.block.clone();
        request.prior_questions = vec![PriorQuestion {
            title: "Seguinte, um amigo te pede dinheiro emprestado toda semana e nunca devolve. O que você faz?".to_string(),
            block_id: "blk_old".to_string(),
            answered_at_ms: 1,
        }];

        let result = classifier.classify(&request).expect("laya");
        assert_eq!(result.cue, SuggestionCue::Question);
        assert_eq!(result.backend, "laya");
        assert_eq!(laya.requests().len(), 1);
    }

    #[test]
    fn a_positive_laya_decision_is_not_overridden_by_a_heuristic() {
        // R23: a guarda não pode trocar uma decisão positiva do modelo por uma
        // regra de palavras. Só `already_answered` (Jaccard) entra antes do Laya.
        let heuristic = Arc::new(FakeSuggestionCueClassifier::new());
        let mut negative = outcome("heuristic");
        negative.cue = SuggestionCue::NoAction;
        negative.should_suggest = false;
        heuristic.push_outcome(negative);

        let laya = Arc::new(FakeSuggestionCueClassifier::new());
        laya.push_outcome(outcome("laya"));

        let classifier = SettingsSuggestionCueClassifier::new(store("laya"), heuristic, Some(laya));
        let result = classifier.classify(&request()).expect("laya");
        assert_eq!(result.cue, SuggestionCue::Question);
        assert!(result.should_suggest);
    }

    #[test]
    fn real_heuristic_guard_marks_a_long_repeated_question_when_laya_says_comment() {
        use crate::adapters::ai::heuristic_suggestion_cue_classifier::HeuristicSuggestionCueClassifier;
        use crate::domain::suggestion_gate::PriorQuestion;

        let laya = Arc::new(FakeSuggestionCueClassifier::new());
        let mut positive = outcome("laya");
        positive.cue = SuggestionCue::Comment;
        laya.push_outcome(positive);

        let classifier = SettingsSuggestionCueClassifier::new(
            store("laya"),
            Arc::new(HeuristicSuggestionCueClassifier::default()),
            Some(laya),
        );

        let repeated = "Pergunta Dissertativa Load balancer. Você precisa construir uma aplicação web que receberá milhões de requisições por dia e terá várias instâncias do back-end rodando em paralelo. Onde você colocaria o load balancer na arquitetura e quais ferramentas ou tecnologias utilizaria para implementá-lo? Explique também a diferença entre usar um load balancer gerenciado na nuvem e uma solução própria.";
        let mut request = request();
        request.block.turns = vec![BlockTurn {
            speaker: Speaker::They,
            text: repeated.to_string(),
            start_ms: 0,
            end_ms: 40_000,
        }];
        request.block.end_ms = 40_000;
        request.block.they_chars = repeated.chars().count();
        request.full_block = request.block.clone();
        request.prior_questions = vec![PriorQuestion {
            title: "Pergunta Dissertativa Load Balancer. Você precisa construir uma aplicação que receberá milhões de requisições por dia e terá várias instâncias do back-end rodando em paralelo. Onde".to_string(),
            block_id: "blk_old".to_string(),
            answered_at_ms: 1,
        }];

        let result = classifier.classify(&request).expect("guarda");
        assert_eq!(result.cue, SuggestionCue::AlreadyAnswered);
        assert_eq!(result.backend, "heuristic");
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
