//! Porta do classificador de blocos (BR-094).
//!
//! O classificador **decide** se um bloco deve gerar sugestão; o gerador
//! existente continua escrevendo o texto. A porta é síncrona de propósito: o
//! agendador roda em thread própria e o adaptador é responsável por aplicar
//! timeout no backend.
//!
//! TODO(task 03): remover o `allow(dead_code)` quando o gate/serviço consumir a
//! porta; até lá ela é exercitada pelos testes e pelo fake.
#![allow(dead_code)]

use serde::Serialize;

use crate::domain::suggestion_gate::{ConversationBlock, PriorQuestion, SuggestionCue};

/// Pedido de classificação de um bloco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationRequest {
    pub session_id: String,
    /// Alvo da classificação: falas de THEY desde a última fala de ME.
    pub block: ConversationBlock,
    /// Bloco completo selecionado, com ME+THEY (contexto do modelo).
    pub full_block: ConversationBlock,
    /// Blocos anteriores, do mais antigo para o mais novo.
    pub context: Vec<ConversationBlock>,
    /// Idioma da interface/transcrição (`pt`, `en`, …).
    pub locale: String,
    /// Agente selecionado; o classificador pode usar como contexto.
    pub mode_id: String,
    /// Contexto curto do agente (nome/categoria/foco).
    pub mode_context: String,
    /// Classifica bloco de um falante só (escopo adaptativo).
    pub allow_single_speaker: bool,
    /// Perguntas já respondidas na sessão (mais recentes primeiro).
    pub prior_questions: Vec<PriorQuestion>,
    /// Instante da classificação (ms desde o início da sessão).
    pub now_ms: i64,
}

/// Resultado tipado do classificador.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClassificationOutcome {
    pub block_id: String,
    pub session_id: String,
    pub cue: SuggestionCue,
    pub should_suggest: bool,
    /// Confiança calibrada em `0.0..=1.0`.
    pub confidence: f32,
    /// Trecho curto da fala que pesou na decisão (≤ 160 chars).
    pub evidence: String,
    /// Foco curto para o gerador (≤ 240 chars); opcional.
    pub focus: Option<String>,
    /// `heuristic`, `laya`, `fake`, …
    pub backend: String,
    /// `multilingual`, `english`, …
    pub model: Option<String>,
    pub latency_ms: u64,
    pub block_start_ms: i64,
    pub block_end_ms: i64,
    pub turn_count: usize,
    pub classified_at_ms: i64,
    /// `true` quando a decisão veio de abstenção/baixa confiança.
    pub abstained: bool,
}

impl ClassificationOutcome {
    /// Normaliza os campos que a UI/gerador consomem.
    ///
    /// O contrato não confia no backend: confiança inválida vira `0.0`,
    /// evidência/foco são truncados e `should_suggest` nunca é `true` para as
    /// cues negativas.
    pub fn normalized(mut self) -> Self {
        if !self.confidence.is_finite() {
            self.confidence = 0.0;
        }
        self.confidence = self.confidence.clamp(0.0, 1.0);
        self.evidence = truncate_chars(&self.evidence, 160);
        self.focus = self
            .focus
            .map(|focus| truncate_chars(&focus, 240))
            .filter(|focus| !focus.is_empty());
        if !self.cue.can_suggest() {
            self.should_suggest = false;
        }
        self
    }
}

fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

/// Falhas tipadas do classificador.
#[derive(Debug, thiserror::Error)]
pub enum ClassifierError {
    #[error("classificador desligado")]
    Disabled,
    #[error("classificador indisponível")]
    Unavailable,
    #[error("timeout do classificador")]
    Timeout,
    #[error("resposta inválida do classificador")]
    InvalidResponse,
    #[error("erro do classificador: {0}")]
    Other(String),
}

/// Classifica um bloco de conversa.
///
/// Implementações: `HeuristicSuggestionCueClassifier` (offline),
/// `LayaSuggestionCueClassifier` (POC/ONNX) e `FakeSuggestionCueClassifier`
/// (testes).
pub trait SuggestionCueClassifier: Send + Sync {
    fn classify(
        &self,
        request: &ClassificationRequest,
    ) -> std::result::Result<ClassificationOutcome, ClassifierError>;

    /// Identidade do backend para UI/settings; nunca contém segredo.
    fn backend_id(&self) -> &'static str;

    /// Repetição lexical de uma pergunta já respondida (Jaccard de tokens).
    ///
    /// É uma decisão **independente de idioma/palavras-chave** (R23): compara os
    /// tokens do bloco atual com os títulos do histórico. O padrão é `None`; o
    /// heurístico implementa e o decorador usa como guarda determinística do
    /// `already_answered` (o modelo zero-shot não é confiável nisso).
    fn already_answered(&self, _request: &ClassificationRequest) -> Option<ClassificationOutcome> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::suggestion_gate::{BlockTurn, SuggestionCue};
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

    fn outcome() -> ClassificationOutcome {
        ClassificationOutcome {
            block_id: "blk_1".to_string(),
            session_id: "s1".to_string(),
            cue: SuggestionCue::Question,
            should_suggest: true,
            confidence: 0.9,
            evidence: "pode explicar?".to_string(),
            focus: Some("responder à pergunta".to_string()),
            backend: "fake".to_string(),
            model: Some("test".to_string()),
            latency_ms: 1,
            block_start_ms: 0,
            block_end_ms: 1000,
            turn_count: 1,
            classified_at_ms: 1000,
            abstained: false,
        }
    }

    #[test]
    fn request_carries_block_and_context() {
        let request = ClassificationRequest {
            session_id: "s1".to_string(),
            block: block(),
            full_block: block(),
            context: vec![],
            locale: "pt".to_string(),
            mode_id: "default".to_string(),
            mode_context: String::new(),
            allow_single_speaker: false,
            prior_questions: vec![],
            now_ms: 1_000,
        };
        assert_eq!(request.block.id, "blk_1");
        assert_eq!(request.locale, "pt");
        assert!(request.prior_questions.is_empty());
    }

    #[test]
    fn outcome_normalizes_confidence_and_negative_cue() {
        let mut outcome = outcome();
        outcome.confidence = 2.5;
        outcome.evidence = "x".repeat(200);
        outcome.cue = SuggestionCue::NoAction;
        outcome.should_suggest = true;

        let normalized = outcome.normalized();
        assert_eq!(normalized.confidence, 1.0);
        assert_eq!(normalized.evidence.chars().count(), 160);
        assert!(!normalized.should_suggest);
    }

    #[test]
    fn outcome_serializes_snake_case_fields() {
        let json = serde_json::to_value(outcome()).expect("json");
        assert_eq!(json["block_id"], "blk_1");
        assert_eq!(json["should_suggest"], true);
        assert_eq!(json["cue"], "question");
    }
}
