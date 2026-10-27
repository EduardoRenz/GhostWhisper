//! Métricas do eval do classificador (BR-098).
//!
//! Regras puras: recebem pares rotulado × previsto e calculam acurácia de
//! `should_suggest`, P/R/F1 por cue, matriz de confusão e p50/p95. O harness
//! (fixtures reais) vive no adaptador; aqui não há I/O.
//!
//! O harness de POC (ignorado por padrão) usa estas métricas; em produção só
//! o model manager consome o relatório publicado.
#![allow(dead_code)]

use serde::Serialize;

use crate::domain::suggestion_gate::SuggestionCue;

/// Quantidade de cues do classificador.
pub const CUE_COUNT: usize = 5;

/// Posição estável da cue na matriz de confusão.
pub fn cue_index(cue: SuggestionCue) -> usize {
    match cue {
        SuggestionCue::Question => 0,
        SuggestionCue::Comment => 1,
        SuggestionCue::AlreadyAnswered => 2,
        SuggestionCue::NoAction => 3,
        SuggestionCue::InsufficientContext => 4,
    }
}

/// Rótulos na ordem da matriz/tabela.
pub const CUE_LABELS: [&str; CUE_COUNT] = [
    "question",
    "comment",
    "already_answered",
    "no_action",
    "insufficient_context",
];

/// Um exemplo avaliado.
#[derive(Debug, Clone, PartialEq)]
pub struct LabeledPrediction {
    pub expected_cue: SuggestionCue,
    pub predicted_cue: SuggestionCue,
    pub expected_should_suggest: bool,
    pub predicted_should_suggest: bool,
    pub latency_ms: u64,
}

/// P/R/F1 de uma cue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CueScore {
    pub cue: String,
    pub support: usize,
    pub precision: f32,
    pub recall: f32,
    pub f1: f32,
}

/// Relatório do eval.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvalReport {
    pub total: usize,
    /// Acurácia da decisão `should_suggest`.
    pub accuracy: f32,
    /// Falsos positivos / negativos reais.
    pub false_positive_rate: f32,
    /// Falsos negativos / positivos reais.
    pub false_negative_rate: f32,
    /// Acurácia da cue (multi-classe).
    pub cue_accuracy: f32,
    pub per_cue: Vec<CueScore>,
    /// `confusion[esperado][previsto]`.
    pub confusion: Vec<Vec<usize>>,
    pub p50_ms: u64,
    pub p95_ms: u64,
}

/// Calcula o relatório; entradas vazias devolvem métricas zeradas.
pub fn evaluate(items: &[LabeledPrediction]) -> EvalReport {
    let total = items.len();
    let mut confusion = vec![vec![0_usize; CUE_COUNT]; CUE_COUNT];
    let mut correct_decision = 0_usize;
    let mut true_positive = 0_usize;
    let mut false_positive = 0_usize;
    let mut true_negative = 0_usize;
    let mut false_negative = 0_usize;
    let mut correct_cue = 0_usize;

    for item in items {
        confusion[cue_index(item.expected_cue)][cue_index(item.predicted_cue)] += 1;
        if item.expected_cue == item.predicted_cue {
            correct_cue += 1;
        }
        match (item.expected_should_suggest, item.predicted_should_suggest) {
            (true, true) => {
                correct_decision += 1;
                true_positive += 1;
            }
            (false, false) => {
                correct_decision += 1;
                true_negative += 1;
            }
            (false, true) => false_positive += 1,
            (true, false) => false_negative += 1,
        }
    }

    let per_cue = (0..CUE_COUNT)
        .map(|index| {
            let support = confusion[index].iter().sum::<usize>();
            let tp = confusion[index][index];
            let predicted = (0..CUE_COUNT)
                .map(|row| confusion[row][index])
                .sum::<usize>();
            let precision = ratio(tp, predicted);
            let recall = ratio(tp, support);
            let f1 = if precision + recall > 0.0 {
                2.0 * precision * recall / (precision + recall)
            } else {
                0.0
            };
            CueScore {
                cue: CUE_LABELS[index].to_string(),
                support,
                precision,
                recall,
                f1,
            }
        })
        .collect();

    EvalReport {
        total,
        accuracy: ratio(correct_decision, total),
        false_positive_rate: ratio(false_positive, false_positive + true_negative),
        false_negative_rate: ratio(false_negative, false_negative + true_positive),
        cue_accuracy: ratio(correct_cue, total),
        per_cue,
        confusion,
        p50_ms: percentile(items, 50),
        p95_ms: percentile(items, 95),
    }
}

fn ratio(part: usize, whole: usize) -> f32 {
    if whole == 0 {
        0.0
    } else {
        part as f32 / whole as f32
    }
}

fn percentile(items: &[LabeledPrediction], percentile: u32) -> u64 {
    if items.is_empty() {
        return 0;
    }
    let mut latencies: Vec<u64> = items.iter().map(|item| item.latency_ms).collect();
    latencies.sort_unstable();
    let rank = ((percentile as f64 / 100.0) * latencies.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(latencies.len() - 1);
    latencies[index]
}

impl EvalReport {
    /// Tabela Markdown para colar no `memory.md`.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "- Exemplos: **{}** · acurácia (`should_suggest`): **{:.1}%** · FP: **{:.1}%** · FN: **{:.1}%**\n",
            self.total,
            self.accuracy * 100.0,
            self.false_positive_rate * 100.0,
            self.false_negative_rate * 100.0
        ));
        out.push_str(&format!(
            "- Acurácia da cue: **{:.1}%** · p50: **{} ms** · p95: **{} ms**\n\n",
            self.cue_accuracy * 100.0,
            self.p50_ms,
            self.p95_ms
        ));
        out.push_str("| cue | suporte | precisão | recall | F1 |\n");
        out.push_str("| --- | ---: | ---: | ---: | ---: |\n");
        for score in &self.per_cue {
            out.push_str(&format!(
                "| {} | {} | {:.2} | {:.2} | {:.2} |\n",
                score.cue, score.support, score.precision, score.recall, score.f1
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(
        expected: SuggestionCue,
        predicted: SuggestionCue,
        latency_ms: u64,
    ) -> LabeledPrediction {
        LabeledPrediction {
            expected_cue: expected,
            predicted_cue: predicted,
            expected_should_suggest: expected.can_suggest(),
            predicted_should_suggest: predicted.can_suggest(),
            latency_ms,
        }
    }

    #[test]
    fn perfect_predictions_score_one() {
        let items: Vec<LabeledPrediction> = [
            SuggestionCue::Question,
            SuggestionCue::Comment,
            SuggestionCue::NoAction,
            SuggestionCue::AlreadyAnswered,
        ]
        .into_iter()
        .map(|cue| item(cue, cue, 10))
        .collect();

        let report = evaluate(&items);
        assert_eq!(report.total, 4);
        assert_eq!(report.accuracy, 1.0);
        assert_eq!(report.cue_accuracy, 1.0);
        assert_eq!(report.false_positive_rate, 0.0);
        assert_eq!(report.false_negative_rate, 0.0);
        assert_eq!(report.p50_ms, 10);
        assert_eq!(report.p95_ms, 10);
        for score in &report.per_cue {
            if score.support > 0 {
                assert_eq!(score.f1, 1.0, "cue {}", score.cue);
            }
        }
    }

    #[test]
    fn confusion_and_false_positive_are_computed() {
        let items = vec![
            item(SuggestionCue::Question, SuggestionCue::Question, 1),
            item(SuggestionCue::NoAction, SuggestionCue::Question, 2),
            item(SuggestionCue::AlreadyAnswered, SuggestionCue::NoAction, 3),
            item(SuggestionCue::Comment, SuggestionCue::Comment, 4),
        ];
        let report = evaluate(&items);

        assert_eq!(
            report.confusion[cue_index(SuggestionCue::Question)]
                [cue_index(SuggestionCue::Question)],
            1
        );
        assert_eq!(
            report.confusion[cue_index(SuggestionCue::NoAction)]
                [cue_index(SuggestionCue::Question)],
            1
        );
        // 3 corretas de 4 decisões.
        assert_eq!(report.accuracy, 0.75);
        // 1 FP (no_action previsto como question) e 1 TN.
        assert_eq!(report.false_positive_rate, 0.5);
        // Nenhum falso negativo.
        assert_eq!(report.false_negative_rate, 0.0);
    }

    #[test]
    fn percentiles_use_the_nearest_rank() {
        let items: Vec<LabeledPrediction> = (1..=100)
            .map(|latency| item(SuggestionCue::Question, SuggestionCue::Question, latency))
            .collect();
        let report = evaluate(&items);
        assert_eq!(report.p50_ms, 50);
        assert_eq!(report.p95_ms, 95);
    }

    #[test]
    fn empty_input_is_safe() {
        let report = evaluate(&[]);
        assert_eq!(report.total, 0);
        assert_eq!(report.accuracy, 0.0);
        assert_eq!(report.p95_ms, 0);
        assert_eq!(report.per_cue.len(), CUE_COUNT);
    }

    #[test]
    fn markdown_has_the_metrics_and_every_cue() {
        let report = evaluate(&[item(SuggestionCue::Question, SuggestionCue::Question, 7)]);
        let markdown = report.to_markdown();
        assert!(markdown.contains("acurácia"));
        assert!(markdown.contains("| question |"));
        assert!(markdown.contains("| insufficient_context |"));
    }
}
