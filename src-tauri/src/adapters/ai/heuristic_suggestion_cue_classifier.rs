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
    heuristic_cue, ConversationBlock, HeuristicConfig, SuggestionCue,
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
        SuggestionCue::Hesitation => 0.80,
        SuggestionCue::Silence => 0.75,
        SuggestionCue::NoSuggestion => 0.60,
        SuggestionCue::InsufficientContext => 0.40,
        // O heurístico não devolve estas cues; valor conservador para completar.
        SuggestionCue::LostThread | SuggestionCue::Invitation => 0.50,
    }
}

fn focus(cue: SuggestionCue) -> Option<String> {
    let text = match cue {
        SuggestionCue::Question => "responder à pergunta",
        SuggestionCue::Hesitation => "continuar a frase",
        SuggestionCue::Silence => "puxar a próxima fala",
        SuggestionCue::LostThread => "retomar o fio",
        SuggestionCue::Invitation => "responder ao convite",
        SuggestionCue::NoSuggestion | SuggestionCue::InsufficientContext => return None,
    };
    Some(text.to_string())
}

fn evidence(block: &ConversationBlock, cue: SuggestionCue) -> String {
    let speaker = match cue {
        SuggestionCue::Question => Some(Speaker::They),
        SuggestionCue::Hesitation | SuggestionCue::Silence => Some(Speaker::Me),
        _ => None,
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
        let cue = heuristic_cue(block, request.now_ms, &self.config);
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
    use crate::domain::suggestion_gate::BlockTurn;

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

    fn request(block: ConversationBlock, now_ms: i64) -> ClassificationRequest {
        ClassificationRequest {
            session_id: "s1".to_string(),
            block,
            context: vec![],
            locale: "pt".to_string(),
            mode_id: "default".to_string(),
            mode_context: String::new(),
            allow_single_speaker: false,
            is_open_probe: false,
            now_ms,
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
            .classify(&request(block, 10_000))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::Question);
        assert!(outcome.should_suggest);
        assert!(outcome.evidence.contains("pode explicar"));
        assert_eq!(outcome.focus.as_deref(), Some("responder à pergunta"));
        assert_eq!(outcome.backend, "heuristic");
        assert_eq!(outcome.confidence, 0.90);
    }

    #[test]
    fn detects_hesitation_with_me_evidence() {
        let classifier = HeuristicSuggestionCueClassifier::default();
        let block = block(
            vec![
                (Speaker::Me, "é, tipo assim, eu acho", 0, 2_000),
                (Speaker::They, "certo", 2_500, 2_800),
            ],
            false,
        );
        let outcome = classifier
            .classify(&request(block, 3_000))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::Hesitation);
        assert!(outcome.evidence.contains("tipo assim"));
        assert_eq!(outcome.focus.as_deref(), Some("continuar a frase"));
        assert_eq!(outcome.confidence, 0.80);
    }

    #[test]
    fn detects_silence_after_me() {
        let classifier = HeuristicSuggestionCueClassifier::default();
        let block = block(
            vec![
                (Speaker::Me, "vamos seguir com o plano", 0, 500),
                (Speaker::They, "certo", 1_000, 1_200),
                (Speaker::Me, "entao podemos fechar assim", 1_500, 2_000),
            ],
            false,
        );
        let outcome = classifier
            .classify(&request(block, 6_000))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::Silence);
        assert_eq!(outcome.focus.as_deref(), Some("puxar a próxima fala"));
        assert_eq!(outcome.confidence, 0.75);
    }

    #[test]
    fn fluent_exchange_is_no_suggestion() {
        let classifier = HeuristicSuggestionCueClassifier::default();
        let block = block(
            vec![
                (Speaker::Me, "vamos revisar o cronograma", 0, 1_000),
                (Speaker::They, "concordo com o plano", 1_500, 2_000),
                (Speaker::Me, "fechamos os proximos passos", 2_500, 3_000),
            ],
            false,
        );
        let outcome = classifier
            .classify(&request(block, 4_000))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::NoSuggestion);
        assert!(!outcome.should_suggest);
        assert!(outcome.focus.is_none());
        assert_eq!(outcome.confidence, 0.60);
    }

    #[test]
    fn short_block_is_insufficient_context_and_abstains() {
        let classifier = HeuristicSuggestionCueClassifier::default();
        let block = block(
            vec![(Speaker::Me, "oi", 0, 100), (Speaker::They, "ok", 200, 300)],
            false,
        );
        let outcome = classifier
            .classify(&request(block, 1_000))
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
