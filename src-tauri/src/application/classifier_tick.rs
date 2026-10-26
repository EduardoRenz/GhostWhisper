//! Escolha do outcome de um tick do modo classificador (BR-092/BR-094).
//!
//! Ordem: sonda de bloco aberto (hesitação) → bloco fechado → fallback. O
//! agendador só gera quando o gate manda; esta função concentra a decisão de
//! **qual** classificação usar e quando cair no timer por erro do classificador.

use crate::ports::suggestion_cue_classifier::ClassificationOutcome;
use crate::ports::suggestion_gate::{SuggestionGate, SuggestionGateError};

/// Resultado da escolha de um tick.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassifierTick {
    /// Há uma classificação para o gate decidir (gerar ou não).
    Classified(ClassificationOutcome),
    /// Nada classificável neste tick (sem bloco / sonda fora do throttle).
    Skip,
    /// O classificador falhou e as settings pedem o caminho automático.
    FallbackAuto,
}

/// Escolhe a classificação do tick.
///
/// - sonda aberta com resultado → usa (hesitação pode gerar com ME falando);
/// - sonda sem resultado → classifica o último bloco fechado;
/// - `NoBlock` → `Skip` (não é falha);
/// - erro do classificador → `FallbackAuto` se configurado, senão `Skip`.
///
/// A sonda e o bloco fechado não rodam no mesmo tick: se a sonda classificou
/// (mesmo negativa), a decisão já foi tomada para o contexto atual.
pub fn classify_once(
    gate: &dyn SuggestionGate,
    session_id: &str,
    fallback_auto: bool,
) -> ClassifierTick {
    // Decisão positiva que ficou no cooldown: reusa antes de classificar de
    // novo. `should_generate` decide se já pode gerar ou se continua pendente.
    if let Some(pending) = gate.take_pending(session_id) {
        return ClassifierTick::Classified(pending);
    }

    let probe = match gate.probe_open(session_id) {
        Ok(outcome) => outcome,
        Err(error) => {
            tracing::debug!("sonda do classificador indisponível: {error}");
            return if fallback_auto {
                ClassifierTick::FallbackAuto
            } else {
                ClassifierTick::Skip
            };
        }
    };
    if let Some(outcome) = probe {
        return ClassifierTick::Classified(outcome);
    }

    match gate.classify_latest(session_id, false) {
        Ok(outcome) => ClassifierTick::Classified(outcome),
        Err(SuggestionGateError::NoBlock) => ClassifierTick::Skip,
        Err(error) => {
            tracing::debug!("classificador indisponível: {error}");
            if fallback_auto {
                ClassifierTick::FallbackAuto
            } else {
                ClassifierTick::Skip
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::suggestion_gate::{GateDecision, SuggestionCue};

    fn outcome(cue: SuggestionCue) -> ClassificationOutcome {
        ClassificationOutcome {
            block_id: "blk_1".to_string(),
            session_id: "s1".to_string(),
            cue,
            should_suggest: cue.can_suggest(),
            confidence: 0.9,
            evidence: "evidência".to_string(),
            focus: None,
            backend: "fake".to_string(),
            model: None,
            latency_ms: 0,
            block_start_ms: 0,
            block_end_ms: 1_000,
            turn_count: 2,
            classified_at_ms: 1_000,
            abstained: false,
        }
    }

    #[derive(Clone)]
    enum Probe {
        Nothing,
        Outcome(ClassificationOutcome),
        Error,
    }

    #[derive(Clone)]
    enum Closed {
        Outcome(ClassificationOutcome),
        NoBlock,
        Error,
    }

    struct FakeGate {
        probe: Probe,
        closed: Closed,
        pending: Option<ClassificationOutcome>,
    }

    impl SuggestionGate for FakeGate {
        fn classify_latest(
            &self,
            _session_id: &str,
            _force_open: bool,
        ) -> std::result::Result<ClassificationOutcome, SuggestionGateError> {
            match &self.closed {
                Closed::Outcome(outcome) => Ok(outcome.clone()),
                Closed::NoBlock => Err(SuggestionGateError::NoBlock),
                Closed::Error => Err(SuggestionGateError::Classifier(
                    crate::ports::suggestion_cue_classifier::ClassifierError::Unavailable,
                )),
            }
        }

        fn probe_open(
            &self,
            _session_id: &str,
        ) -> std::result::Result<Option<ClassificationOutcome>, SuggestionGateError> {
            match &self.probe {
                Probe::Nothing => Ok(None),
                Probe::Outcome(outcome) => Ok(Some(outcome.clone())),
                Probe::Error => Err(SuggestionGateError::Classifier(
                    crate::ports::suggestion_cue_classifier::ClassifierError::Timeout,
                )),
            }
        }

        fn last(&self, _session_id: &str) -> Option<ClassificationOutcome> {
            None
        }

        fn pending(&self, _session_id: &str) -> Option<ClassificationOutcome> {
            None
        }

        fn take_pending(&self, _session_id: &str) -> Option<ClassificationOutcome> {
            self.pending.clone()
        }

        fn should_generate(
            &self,
            _outcome: &ClassificationOutcome,
            _now_ms: i64,
            _generating: bool,
        ) -> GateDecision {
            GateDecision::Generate
        }

        fn mark_generated(&self, _session_id: &str, _block_id: &str, _at_ms: i64) {}

        fn reset(&self, _session_id: &str) {}
    }

    #[test]
    fn prefers_the_open_probe() {
        let gate = FakeGate {
            probe: Probe::Outcome(outcome(SuggestionCue::Hesitation)),
            closed: Closed::Outcome(outcome(SuggestionCue::Question)),
            pending: None,
        };
        assert_eq!(
            classify_once(&gate, "s1", false),
            ClassifierTick::Classified(outcome(SuggestionCue::Hesitation))
        );
    }

    #[test]
    fn uses_the_closed_block_when_the_probe_has_nothing() {
        let gate = FakeGate {
            probe: Probe::Nothing,
            closed: Closed::Outcome(outcome(SuggestionCue::Question)),
            pending: None,
        };
        assert_eq!(
            classify_once(&gate, "s1", false),
            ClassifierTick::Classified(outcome(SuggestionCue::Question))
        );
    }

    #[test]
    fn no_block_is_skip_not_failure() {
        let gate = FakeGate {
            probe: Probe::Nothing,
            closed: Closed::NoBlock,
            pending: None,
        };
        assert_eq!(classify_once(&gate, "s1", false), ClassifierTick::Skip);
        assert_eq!(classify_once(&gate, "s1", true), ClassifierTick::Skip);
    }

    #[test]
    fn probe_error_skips_by_default_and_falls_back_when_configured() {
        let gate = FakeGate {
            probe: Probe::Error,
            closed: Closed::Outcome(outcome(SuggestionCue::Question)),
            pending: None,
        };
        assert_eq!(classify_once(&gate, "s1", false), ClassifierTick::Skip);
        assert_eq!(
            classify_once(&gate, "s1", true),
            ClassifierTick::FallbackAuto
        );
    }

    #[test]
    fn reuses_a_pending_positive_decision_before_classifying() {
        let gate = FakeGate {
            probe: Probe::Nothing,
            closed: Closed::NoBlock,
            pending: Some(outcome(SuggestionCue::Hesitation)),
        };
        assert_eq!(
            classify_once(&gate, "s1", false),
            ClassifierTick::Classified(outcome(SuggestionCue::Hesitation))
        );
    }

    #[test]
    fn closed_error_skips_by_default_and_falls_back_when_configured() {
        let gate = FakeGate {
            probe: Probe::Nothing,
            closed: Closed::Error,
            pending: None,
        };
        assert_eq!(classify_once(&gate, "s1", false), ClassifierTick::Skip);
        assert_eq!(
            classify_once(&gate, "s1", true),
            ClassifierTick::FallbackAuto
        );
    }
}
