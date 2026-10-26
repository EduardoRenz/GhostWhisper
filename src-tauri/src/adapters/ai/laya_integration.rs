//! Testes de integração do Laya com o artefato ONNX real.
//!
//! Não rodam no `cargo test` normal (exigem `--features laya` + modelo). Com
//! `LAYA_ONNX_DIR` apontando para `model.onnx`/`model.onnx.data`, executam as
//! 28 fixtures uma a uma e imprimem a decisão de cada situação.
//!
//! ```bash
//! cd src-tauri
//! LAYA_ONNX_DIR=$PWD/resources/laya/multilingual \
//!   cargo test --lib laya_integration --features laya -- --ignored --nocapture
//! ```
//!
//! O teste **não falha** por qualidade: ele é um relatório. Para aplicar o gate
//! (≥80% / ≤15%), rode com `LAYA_ENFORCE_GATE=1`; hoje ele falha de propósito
//! enquanto o modelo zero-shot não estiver fine-tuned (ISS-062).

use std::path::Path;

use serde::Deserialize;

use crate::adapters::ai::laya_onnx_predictor::OnnxLayaPredictor;
use crate::adapters::ai::laya_suggestion_cue_classifier::{
    LayaClassifierConfig, LayaSuggestionCueClassifier,
};
use crate::domain::classifier_eval::{evaluate, LabeledPrediction};
use crate::domain::settings::Settings;
use crate::domain::suggestion_gate::{BlockTurn, ConversationBlock, SuggestionCue};
use crate::domain::transcription::Speaker;
use crate::ports::suggestion_cue_classifier::{ClassificationRequest, SuggestionCueClassifier};

/// Contexto de agente equivalente ao usado no app (Entrevista Técnica).
const AGENT_CONTEXT: &str =
    "Agente: Entrevista Técnica (engineering)\nFoco do agente: responda como em uma entrevista técnica.";

#[derive(Debug, Deserialize)]
struct Fixture {
    id: String,
    locale: String,
    state: String,
    #[serde(default)]
    context: String,
    #[serde(default)]
    gap_ms: i64,
    label_cue: String,
    label_should_suggest: bool,
}

fn load_fixtures() -> Vec<Fixture> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../poc/laya-classifier/fixtures/blocks.jsonl");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixtures em {}: {error}", path.display()));
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("fixture"))
        .collect()
}

fn turns_from_state(state: &str) -> Vec<BlockTurn> {
    state
        .lines()
        .filter_map(|line| {
            let (speaker, text) = line.split_once(": ")?;
            let speaker = match speaker.trim() {
                "ME" => Speaker::Me,
                "THEY" => Speaker::They,
                _ => return None,
            };
            Some((speaker, text.trim()))
        })
        .enumerate()
        .map(|(index, (speaker, text))| {
            let start = index as i64 * 1_000;
            BlockTurn {
                speaker,
                text: text.to_string(),
                start_ms: start,
                end_ms: start + 900,
            }
        })
        .collect()
}

fn conversation(state: &str, id: &str) -> ConversationBlock {
    let turns = turns_from_state(state);
    ConversationBlock {
        id: id.to_string(),
        session_id: "s1".to_string(),
        start_ms: turns.first().map(|turn| turn.start_ms).unwrap_or(0),
        end_ms: turns.last().map(|turn| turn.end_ms).unwrap_or(0),
        me_chars: turns
            .iter()
            .filter(|turn| turn.speaker == Speaker::Me)
            .map(|turn| turn.text.chars().count())
            .sum(),
        they_chars: turns
            .iter()
            .filter(|turn| turn.speaker == Speaker::They)
            .map(|turn| turn.text.chars().count())
            .sum(),
        turns,
        is_open: false,
    }
}

fn request(fixture: &Fixture) -> ClassificationRequest {
    let block = conversation(&fixture.state, &fixture.id);
    let context = if fixture.context.trim().is_empty() {
        vec![]
    } else {
        vec![conversation(&fixture.context, "ctx")]
    };
    ClassificationRequest {
        session_id: "s1".to_string(),
        block,
        context,
        locale: fixture.locale.clone(),
        mode_id: "technical_interview".to_string(),
        mode_context: AGENT_CONTEXT.to_string(),
        allow_single_speaker: true,
        is_open_probe: false,
        now_ms: (fixture.state.lines().count() as i64)
            .saturating_sub(1)
            .saturating_mul(1_000)
            .saturating_add(900)
            .saturating_add(fixture.gap_ms)
            .saturating_add(100),
    }
}

fn classifier(directory: &str) -> LayaSuggestionCueClassifier {
    let predictor = OnnxLayaPredictor::new(directory).expect("modelo Laya");
    // Mesma configuração do app: backend laya + sensibilidade balanceada.
    let settings = Settings {
        suggestion_classifier_backend: "laya".to_string(),
        suggestion_classifier_sensitivity: "balanced".to_string(),
        suggestion_classifier_timeout_ms: 10_000,
        ..Settings::default()
    };
    LayaSuggestionCueClassifier::new(
        Box::new(predictor),
        LayaClassifierConfig::from_settings(&settings),
    )
}

fn model_dir() -> Option<String> {
    match std::env::var("LAYA_ONNX_DIR") {
        Ok(directory) => Some(directory),
        Err(_) => {
            eprintln!(
                "LAYA_ONNX_DIR não definido; rode com o artefato para executar a integração."
            );
            None
        }
    }
}

/// Relatório por situação: imprime cue/confiança/decisão de cada fixture.
#[test]
#[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
fn laya_integration_reports_each_situation() {
    let Some(directory) = model_dir() else {
        return;
    };
    let classifier = classifier(&directory);
    let fixtures = load_fixtures();
    let mut predictions = Vec::new();

    println!(
        "{:<4} {:<18} {:<18} {:>6} {:<8} {:<8} resultado",
        "id", "esperado", "previsto", "conf", "esp_s", "got_s"
    );
    println!("{}", "-".repeat(88));

    for fixture in &fixtures {
        let outcome = classifier.classify(&request(fixture)).expect("classifica");
        let expected_cue = SuggestionCue::from_str(&fixture.label_cue).expect("cue");
        let cue_ok = outcome.cue == expected_cue;
        let decision_ok = outcome.should_suggest == fixture.label_should_suggest;
        let result = if cue_ok && decision_ok {
            "ok"
        } else if cue_ok {
            "decisão"
        } else if decision_ok {
            "cue"
        } else {
            "falhou"
        };
        println!(
            "{:<4} {:<18} {:<18} {:>6} {:<8} {:<8} {}",
            fixture.id,
            expected_cue.as_str(),
            outcome.cue.as_str(),
            format!("{:.2}", outcome.confidence),
            fixture.label_should_suggest,
            outcome.should_suggest,
            result
        );
        predictions.push(LabeledPrediction {
            expected_cue,
            predicted_cue: outcome.cue,
            expected_should_suggest: fixture.label_should_suggest,
            predicted_should_suggest: outcome.should_suggest,
            latency_ms: outcome.latency_ms,
        });
    }

    let report = evaluate(&predictions);
    println!("\n{}", serde_json::to_string_pretty(&report).expect("json"));
    println!("\n{}", report.to_markdown());

    // Relatório por padrão; `LAYA_ENFORCE_GATE=1` transforma em gate rígido.
    if std::env::var("LAYA_ENFORCE_GATE").is_ok() {
        assert!(
            report.accuracy >= 0.80,
            "acurácia {:.1}% < 80% (ISS-062; fine-tuning pendente)",
            report.accuracy * 100.0
        );
        assert!(
            report.false_positive_rate <= 0.15,
            "FP {:.1}% > 15%",
            report.false_positive_rate * 100.0
        );
    }
}
