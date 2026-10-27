//! Classificador heurístico local (sem IA), BR-094.
//!
//! Usa as regras puras de `domain::suggestion_gate::heuristic_cue` e devolve o
//! contrato `ClassificationOutcome`. Serve de fallback offline e de base para
//! testar o gate antes do Laya.
//!
//! TODO(task 10): remover o `allow(dead_code)` quando o scheduler ligar o
//! backend no composition root.
#![allow(dead_code)]

use crate::domain::suggestion_gate::{
    heuristic_cue, is_repeated_question, question_text, ConversationBlock, HeuristicConfig,
    SuggestionCue,
};
use crate::domain::transcription::Speaker;
use crate::ports::suggestion_cue_classifier::{
    ClassificationOutcome, ClassificationRequest, ClassifierError, SuggestionCueClassifier,
};

/// Backend determinístico que não usa modelo.
pub struct HeuristicSuggestionCueClassifier {
    config: HeuristicConfig,
}

impl HeuristicSuggestionCueClassifier {
    pub fn new(config: HeuristicConfig) -> Self {
        Self { config }
    }
}

impl Default for HeuristicSuggestionCueClassifier {
    fn default() -> Self {
        Self::new(HeuristicConfig::default())
    }
}

fn confidence(cue: SuggestionCue) -> f32 {
    match cue {
        SuggestionCue::Question => 0.90,
        SuggestionCue::Comment => 0.86,
        SuggestionCue::AlreadyAnswered => 0.90,
        SuggestionCue::NoAction => 0.60,
        SuggestionCue::InsufficientContext => 0.40,
    }
}

fn focus(cue: SuggestionCue) -> Option<String> {
    let text = match cue {
        SuggestionCue::Question => "responder à pergunta",
        SuggestionCue::Comment => "comentar o ponto",
        SuggestionCue::AlreadyAnswered
        | SuggestionCue::NoAction
        | SuggestionCue::InsufficientContext => return None,
    };
    Some(text.to_string())
}

fn evidence(block: &ConversationBlock, cue: SuggestionCue) -> String {
    let speaker = match cue {
        SuggestionCue::Question | SuggestionCue::Comment | SuggestionCue::AlreadyAnswered => {
            Some(Speaker::They)
        }
        SuggestionCue::NoAction | SuggestionCue::InsufficientContext => None,
    };
    let turn = speaker
        .and_then(|speaker| {
            block
                .turns
                .iter()
                .rev()
                .find(|turn| turn.speaker == speaker)
        })
        .or_else(|| block.turns.last());
    turn.map(|turn| turn.text.clone()).unwrap_or_default()
}

impl SuggestionCueClassifier for HeuristicSuggestionCueClassifier {
    fn classify(
        &self,
        request: &ClassificationRequest,
    ) -> std::result::Result<ClassificationOutcome, ClassifierError> {
        let block = &request.block;
        let mut cue = heuristic_cue(block, &self.config, &request.prior_questions);

        // O alvo pode ser só o último fragmento de THEY. Para "já respondida",
        // compara também a pergunta completa do bloco (com todos os turnos de
        // THEY), senão uma pergunta fragmentada escapa do histórico.
        if matches!(cue, SuggestionCue::Question | SuggestionCue::Comment) {
            let they_text = request
                .full_block
                .turns
                .iter()
                .filter(|turn| turn.speaker == Speaker::They)
                .map(|turn| turn.text.trim())
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let mut candidates: Vec<String> = Vec::new();
            if let Some(question) = question_text(&request.full_block) {
                candidates.push(question);
            }
            if !they_text.is_empty() {
                candidates.push(they_text);
            }
            if candidates.iter().any(|candidate| {
                is_repeated_question(candidate, &request.prior_questions, &self.config)
            }) {
                cue = SuggestionCue::AlreadyAnswered;
            }
        }
        Ok(ClassificationOutcome {
            block_id: block.id.clone(),
            session_id: block.session_id.clone(),
            cue,
            should_suggest: cue.can_suggest(),
            confidence: confidence(cue),
            evidence: evidence(block, cue),
            focus: focus(cue),
            backend: "heuristic".to_string(),
            model: None,
            latency_ms: 0,
            block_start_ms: block.start_ms,
            block_end_ms: block.end_ms,
            turn_count: block.turns.len(),
            classified_at_ms: request.now_ms,
            abstained: cue == SuggestionCue::InsufficientContext,
        }
        .normalized())
    }

    fn backend_id(&self) -> &'static str {
        "heuristic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::suggestion_gate::{BlockTurn, PriorQuestion};

    fn block(turns: Vec<(Speaker, &str, i64, i64)>, is_open: bool) -> ConversationBlock {
        ConversationBlock {
            id: "blk_1".to_string(),
            session_id: "s1".to_string(),
            start_ms: turns.first().map(|turn| turn.2).unwrap_or(0),
            end_ms: turns.last().map(|turn| turn.3).unwrap_or(0),
            turns: turns
                .into_iter()
                .map(|(speaker, text, start_ms, end_ms)| BlockTurn {
                    speaker,
                    text: text.to_string(),
                    start_ms,
                    end_ms,
                })
                .collect(),
            me_chars: 20,
            they_chars: 20,
            is_open,
        }
    }

    fn request(
        block: ConversationBlock,
        prior_questions: Vec<PriorQuestion>,
    ) -> ClassificationRequest {
        ClassificationRequest {
            session_id: "s1".to_string(),
            block: block.clone(),
            full_block: block,
            context: vec![],
            locale: "pt".to_string(),
            mode_id: "default".to_string(),
            mode_context: String::new(),
            allow_single_speaker: true,
            prior_questions,
            now_ms: 10_000,
        }
    }

    fn prior(title: &str) -> PriorQuestion {
        PriorQuestion {
            title: title.to_string(),
            block_id: "blk_old".to_string(),
            answered_at_ms: 1_000,
        }
    }

    #[test]
    fn detects_question_with_they_evidence() {
        let classifier = HeuristicSuggestionCueClassifier::default();
        let block = block(
            vec![
                (Speaker::Me, "eu consigo explicar", 0, 500),
                (Speaker::They, "pode explicar melhor?", 1_000, 1_500),
            ],
            false,
        );
        let outcome = classifier
            .classify(&request(block, vec![]))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::Question);
        assert!(outcome.should_suggest);
        assert!(outcome.evidence.contains("pode explicar"));
        assert_eq!(outcome.focus.as_deref(), Some("responder à pergunta"));
        assert_eq!(outcome.backend, "heuristic");
        assert_eq!(outcome.confidence, 0.90);
    }

    #[test]
    fn detects_comment_invitation() {
        let classifier = HeuristicSuggestionCueClassifier::default();
        let block = block(
            vec![
                (Speaker::Me, "podemos priorizar a migração", 0, 500),
                (
                    Speaker::They,
                    "eu acho que o banco é o gargalo. o que você acha",
                    1_000,
                    1_800,
                ),
            ],
            false,
        );
        let outcome = classifier
            .classify(&request(block, vec![]))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::Comment);
        assert!(outcome.should_suggest);
        assert_eq!(outcome.focus.as_deref(), Some("comentar o ponto"));
        assert_eq!(outcome.confidence, 0.86);
    }

    #[test]
    fn repeated_question_is_already_answered() {
        let classifier = HeuristicSuggestionCueClassifier::default();
        let block = block(
            vec![
                (Speaker::Me, "o desconto é 10%", 0, 500),
                (
                    Speaker::They,
                    "e o desconto para 50 licenças, como fica?",
                    1_000,
                    1_500,
                ),
            ],
            false,
        );
        let outcome = classifier
            .classify(&request(
                block,
                vec![prior("e o desconto para 50 licenças, como fica?")],
            ))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::AlreadyAnswered);
        assert!(!outcome.should_suggest);
        assert!(outcome.focus.is_none());
    }

    #[test]
    fn question_answered_in_the_same_block_is_already_answered() {
        let classifier = HeuristicSuggestionCueClassifier::default();
        let block = block(
            vec![
                (Speaker::They, "qual é o prazo?", 0, 500),
                (Speaker::Me, "entrego na sexta", 1_000, 1_500),
            ],
            false,
        );
        let outcome = classifier
            .classify(&request(block, vec![]))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::AlreadyAnswered);
        assert!(!outcome.should_suggest);
    }

    #[test]
    fn casual_conversation_and_explanations_are_no_action() {
        let classifier = HeuristicSuggestionCueClassifier::default();
        let casual = block(
            vec![
                (Speaker::Me, "bom dia, tudo bem?", 0, 500),
                (Speaker::They, "tudo ótimo, e você?", 1_000, 1_500),
            ],
            false,
        );
        let outcome = classifier
            .classify(&request(casual, vec![]))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::NoAction);
        assert!(!outcome.should_suggest);

        let explanation = block(
            vec![
                (Speaker::Me, "como funciona?", 0, 500),
                (
                    Speaker::They,
                    "o sistema usa filas para desacoplar os serviços e retenta falhas.",
                    1_000,
                    3_000,
                ),
            ],
            false,
        );
        let outcome = classifier
            .classify(&request(explanation, vec![]))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::NoAction);
        assert!(outcome.focus.is_none());
    }

    #[test]
    fn short_block_is_insufficient_context_and_abstains() {
        let classifier = HeuristicSuggestionCueClassifier::default();
        let block = block(
            vec![(Speaker::Me, "oi", 0, 100), (Speaker::They, "ok", 200, 300)],
            false,
        );
        let outcome = classifier
            .classify(&request(block, vec![]))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::InsufficientContext);
        assert!(!outcome.should_suggest);
        assert!(outcome.abstained);
        assert!(outcome.focus.is_none());
        assert_eq!(outcome.confidence, 0.40);
    }

    #[test]
    fn backend_id_is_heuristic() {
        assert_eq!(
            HeuristicSuggestionCueClassifier::default().backend_id(),
            "heuristic"
        );
    }
}
