//! Testes de integração do Laya com o artefato ONNX real.
//!
//! Não rodam no `cargo test` normal (exigem `--features laya` + modelo). Com
//! `LAYA_ONNX_DIR` apontando para `model.onnx`/`model.onnx.data`, executam as
//! fixtures uma a uma e imprimem a decisão de cada situação. O classificador
//! testado é o **efetivo do app**: Laya + a guarda lexical de `already_answered`
//! (Jaccard, independente de idioma — R23).
//!
//! ```bash
//! cd src-tauri
//! LAYA_ONNX_DIR=$PWD/resources/laya/multilingual \
//!   cargo test --lib laya_integration --features laya -- --ignored --nocapture
//! ```
//!
//! O teste **não falha** por qualidade: ele é um relatório. Para aplicar o gate
//! (≥80% de decisão / ≤15% de FP), rode com `LAYA_ENFORCE_GATE=1`. A métrica
//! crua do modelo (sem a guarda) continua registrada no README do POC e é o
//! insumo do fine-tuning (ISS-062).

use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;

use crate::adapters::ai::heuristic_suggestion_cue_classifier::HeuristicSuggestionCueClassifier;
use crate::adapters::ai::laya_onnx_predictor::OnnxLayaPredictor;
use crate::adapters::ai::laya_suggestion_cue_classifier::{
    LayaClassifierConfig, LayaSuggestionCueClassifier,
};
use crate::adapters::ai::settings_suggestion_cue_classifier::SettingsSuggestionCueClassifier;
use crate::adapters::settings::memory::InMemorySettingsStore;
use crate::domain::classifier_eval::{evaluate, LabeledPrediction};
use crate::domain::settings::Settings;
use crate::domain::suggestion_gate::{BlockTurn, ConversationBlock, SuggestionCue};
use crate::domain::transcription::Speaker;
use crate::ports::settings_store::SettingsStore;
use crate::ports::suggestion_cue_classifier::{ClassificationRequest, SuggestionCueClassifier};

/// Contexto de agente equivalente ao usado no app (Entrevista Técnica).
const AGENT_CONTEXT: &str = "Agent: Entrevista Técnica (engineering)";

#[derive(Debug, Deserialize)]
struct Fixture {
    id: String,
    locale: String,
    state: String,
    #[serde(default)]
    context: String,
    #[serde(default)]
    gap_ms: i64,
    #[serde(default)]
    prior_questions: Vec<String>,
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
    let full_block = conversation(&fixture.state, &fixture.id);
    let block = crate::domain::suggestion_gate::they_target(&full_block, 3)
        .unwrap_or_else(|| full_block.clone());
    let context = if fixture.context.trim().is_empty() {
        vec![]
    } else {
        vec![conversation(&fixture.context, "ctx")]
    };
    ClassificationRequest {
        session_id: "s1".to_string(),
        block,
        full_block,
        context,
        locale: fixture.locale.clone(),
        mode_id: "technical_interview".to_string(),
        mode_context: AGENT_CONTEXT.to_string(),
        allow_single_speaker: true,
        prior_questions: fixture
            .prior_questions
            .iter()
            .map(|title| crate::domain::suggestion_gate::PriorQuestion {
                title: title.clone(),
                block_id: "blk_prior".to_string(),
                answered_at_ms: 0,
            })
            .collect(),
        now_ms: (fixture.state.lines().count() as i64)
            .saturating_sub(1)
            .saturating_mul(1_000)
            .saturating_add(900)
            .saturating_add(fixture.gap_ms)
            .saturating_add(100),
    }
}

fn request_with_prior(
    state: &str,
    id: &str,
    prior_questions: Vec<crate::domain::suggestion_gate::PriorQuestion>,
) -> ClassificationRequest {
    let full_block = conversation(state, id);
    let block = crate::domain::suggestion_gate::they_target(&full_block, 3)
        .unwrap_or_else(|| full_block.clone());
    ClassificationRequest {
        session_id: "s1".to_string(),
        block,
        full_block,
        context: vec![],
        locale: "pt".to_string(),
        mode_id: "technical_interview".to_string(),
        mode_context: AGENT_CONTEXT.to_string(),
        allow_single_speaker: true,
        prior_questions,
        now_ms: 10_000,
    }
}

fn classifier(directory: &str) -> SettingsSuggestionCueClassifier {
    let predictor = OnnxLayaPredictor::new(directory).expect("modelo Laya");
    // Mesma configuração do app: backend laya + sensibilidade balanceada +
    // guarda determinística das regras locais (já respondida/abstenção).
    let settings = Settings {
        suggestion_classifier_backend: "laya".to_string(),
        suggestion_classifier_sensitivity: "balanced".to_string(),
        suggestion_classifier_timeout_ms: 30_000,
        ..Settings::default()
    };
    let laya = Arc::new(LayaSuggestionCueClassifier::new(
        Box::new(predictor),
        LayaClassifierConfig::from_settings(&settings),
    ));
    let heuristic = Arc::new(HeuristicSuggestionCueClassifier::default());
    let store: Arc<dyn SettingsStore> = Arc::new(InMemorySettingsStore::with_settings(settings));
    SettingsSuggestionCueClassifier::new(store, heuristic, Some(laya))
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
        "{:<4} {:<18} {:<18} {:>6} {:<8} {:<8} {:<10} resultado",
        "id", "esperado", "previsto", "conf", "esp_s", "got_s", "backend"
    );
    println!("{}", "-".repeat(100));

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
            "{:<4} {:<18} {:<18} {:>6} {:<8} {:<8} {:<10} {}",
            fixture.id,
            expected_cue.as_str(),
            outcome.cue.as_str(),
            format!("{:.2}", outcome.confidence),
            fixture.label_should_suggest,
            outcome.should_suggest,
            outcome.backend,
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

/// Regressão da sessão real de 2026-10-26: a mesma pergunta do mutex/fila foi
/// feita em dois blocos consecutivos. O segundo deve cair em `already_answered`
/// mesmo que o Laya tente classificá-lo como pergunta/comentário.
/// Monta a requisição como o app: blocos reais, alvo atravessando blocos
/// contíguos, contexto anterior.
fn reported_request(segments: &[(&str, Speaker, &str, i64, i64)]) -> ClassificationRequest {
    let segments: Vec<crate::domain::transcription::TranscriptSegment> = segments
        .iter()
        .map(
            |(id, speaker, text, start, end)| crate::domain::transcription::TranscriptSegment {
                id: (*id).to_string(),
                session_id: "s1".to_string(),
                speaker: *speaker,
                text: (*text).to_string(),
                start_ms: *start,
                end_ms: *end,
                is_final: true,
            },
        )
        .collect();
    let config = crate::domain::suggestion_gate::BlockConfig {
        gap_ms: 1_000,
        max_block_ms: 45_000,
        max_turns: 12,
        min_chars: 1,
        context_blocks: 8,
    };
    let now = segments.iter().map(|s| s.end_ms).max().unwrap_or(0) + 1_000;
    let blocks = crate::domain::suggestion_gate::build_blocks("s1", &segments, &config, now);
    let index = blocks.len().saturating_sub(1);
    let (target, full, start) = crate::domain::suggestion_gate::target_across_blocks(
        &blocks,
        index,
        3,
        config.max_turns,
        config.gap_ms,
    )
    .unwrap_or_else(|| {
        let target = crate::domain::suggestion_gate::they_target(&blocks[index], 3)
            .unwrap_or_else(|| blocks[index].clone());
        (target, blocks[index].clone(), index)
    });
    let context_start = start.saturating_sub(8);
    ClassificationRequest {
        session_id: "s1".to_string(),
        block: target,
        full_block: full,
        context: blocks[context_start..start].to_vec(),
        locale: "pt".to_string(),
        mode_id: "technical_interview".to_string(),
        mode_context: AGENT_CONTEXT.to_string(),
        allow_single_speaker: true,
        prior_questions: vec![],
        now_ms: now,
    }
}

/// Regressão da sessão `0e882d0a…` (2026-10-27 19:41): a pergunta de arquitetura
/// veio em 4 turnos de THEY e o `build_blocks` divide no limite de tempo. O alvo
/// atravessa os blocos contíguos e o Laya (prompt multilíngue, sem palavras-
/// chave) deve classificar como `question`.
#[test]
#[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
fn regression_0e882d0a_architecture_question_is_question() {
    let Some(directory) = model_dir() else {
        return;
    };
    let classifier = classifier(&directory);
    let segments = [
        ("me", Speaker::Me, "Muito bem, vamos começar então.", 50, 8_966),
        ("they0", Speaker::They, "e aí", 1_907, 13_811),
        ("they1", Speaker::They, "Pergunta de certativa. Arquitetura de software. Imagine que você é o tech lead de uma aplicação financeira que processa transferências bancárias. Dois clientes tentam transferir dinheiro da mesma conta ao mesmo tempo. E o sistema não pode permitir que o saldo fique negativo, nem que uma transferência seja processada.", 13_919, 38_919),
        ("they2", Speaker::They, "como você projetaria essa...", 38_919, 41_951),
        ("they3", Speaker::They, "a solução para garantir consistência dos dados e evitar problemas de concorrência. Responda como se estivesse em uma entrevista técnica. Depois, vou avaliar sua resposta e sugerir melhorias.", 41_951, 53_983),
    ];
    let request = reported_request(&segments);
    let outcome = classifier.classify(&request).expect("classificação");
    assert_eq!(outcome.cue, SuggestionCue::Question);
    assert!(outcome.should_suggest);
    assert!(
        request.block.turns.len() >= 4,
        "o alvo precisa ter os 4 turnos de THEY"
    );
}

/// Regressão da sessão `606de489…`: a pergunta do load balancer também veio
/// partida em turnos; o Laya deve classificar como `question`.
#[test]
#[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
fn regression_606de489_load_balancer_question_is_question() {
    let Some(directory) = model_dir() else {
        return;
    };
    let classifier = classifier(&directory);
    let segments = [
        ("me0", Speaker::Me, "Vamos lá, vamos para o meu sete-over.", 90, 4_142),
        ("they0", Speaker::They, "Pergunta Dissertativa. Load balancer. Você precisa construir uma aplicação web", 1_963, 26_963),
        ("me1", Speaker::Me, "vamos começar a ouvir.", 7_214, 9_433),
        ("they1", Speaker::They, "receberá milhões de requisições por dia e terá várias instâncias do back-end rodando em paralelo.", 26_963, 33_963),
        ("they2", Speaker::They, "o load balancer na arquitetura e quais ferramentas ou tecnologias utilizaria para implementá-lo. Explique também a diferença entre usar um load balancer gerenciado na nuvem, como AWS LB, e uma solução própria, como Enginix ou HAPROXY.", 33_963, 51_883),
    ];
    let request = reported_request(&segments);
    let outcome = classifier.classify(&request).expect("classificação");
    assert_eq!(outcome.cue, SuggestionCue::Question);
    assert!(outcome.should_suggest);
}

/// Regressão da sessão `5984de1d…` (22:02): a última pergunta de arquitetura
/// veio em 3 finais (t5/t6/t7) e o ME só falou "deixa eu pensar" no fim.
/// O alvo atravessa os blocos contíguos e o Laya deve classificar `question`.
#[test]
#[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
fn regression_5984de1d_architecture_question_is_question() {
    let Some(directory) = model_dir() else {
        return;
    };
    let classifier = classifier(&directory);
    let segments = [
        ("me0", Speaker::Me, "Muito bem, então vamos lá, vamos começar a nossa...", 99, 5_431),
        ("me1", Speaker::Me, "Na entrevista estou pronto para algumas perguntas.", 5_431, 9_826),
        ("they5", Speaker::They, "Pergunta dissertativa, arquitetura de software, imagine que você é o tech-lead de uma aplicação financeira que processa transferências bancárias.", 75_969, 100_969),
        ("they6", Speaker::They, "Dois clientes tentam transferir dinheiro da mesma conta ao mesmo tempo, e o sistema não pode permitir que o saldo fique negativo, nem que uma transferência seja processada duas vezes.", 100_969, 113_985),
        ("they7", Speaker::They, "essa solução para garantir consistência dos dados e evitar problemas de concorrência. Responda como se estivesse em uma entrevista técnica. Depois, vou avaliar sua resposta e sugerir melhorias.", 113_985, 127_937),
        ("me10", Speaker::Me, "Está bom, deixa eu pensar aqui então.", 122_765, 127_373),
    ];
    let mut request = reported_request(&segments);
    // O app já tinha a pergunta da carteira no histórico de respondidas.
    let wallet = "Claro, vamos lá. Primeira pergunta. Se você encontra uma carteira no chão o que você faz? A. Pega pra você. B. Entrega pra polícia. C. Tenta achar o dono pelas redes sociais.";
    request.prior_questions = vec![crate::domain::suggestion_gate::PriorQuestion {
        title: wallet.chars().take(180).collect(),
        block_id: "blk_wallet".to_string(),
        answered_at_ms: 1,
    }];
    let outcome = classifier.classify(&request).expect("classificação");
    assert_eq!(outcome.cue, SuggestionCue::Question);
    assert!(outcome.should_suggest);
}

/// A pergunta do amigo/dinheiro da mesma sessão também é uma pergunta.
#[test]
#[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
fn regression_5984de1d_friend_money_question_is_question() {
    let Some(directory) = model_dir() else {
        return;
    };
    let classifier = classifier(&directory);
    let segments = [
        ("they2", Speaker::They, "Beleza. Seguinte, um amigo te pede dinheiro emprestado toda semana e nunca devolve. O que você faz?", 41_921, 66_921),
        ("they3", Speaker::They, "Continuem prestando porque é seu amigo.", 66_921, 69_953),
        ("they4", Speaker::They, "de emprestar e conversa com ele com sinceridade. C, ignora e muda de assunto.", 69_953, 75_969),
    ];
    let request = reported_request(&segments);
    let outcome = classifier.classify(&request).expect("classificação");
    assert_eq!(outcome.cue, SuggestionCue::Question);
    assert!(outcome.should_suggest);
}

/// Regressão da sessão `5f8595a7…` (2026-10-28): a pergunta do load balancer
/// voltou começando no meio (`web que receberá...`) e o histórico tinha só a
/// segunda metade. A guarda lexical tem de sobrescrever o Laya (ISS-081).
#[test]
#[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
fn regression_5f8595a7_load_balancer_repeat_is_already_answered() {
    let Some(directory) = model_dir() else {
        return;
    };
    let classifier = classifier(&directory);
    let segments = [
        ("t8", Speaker::They, "você precisa construir uma aplicação", 172_071, 174_071),
        ("t9", Speaker::They, "web que receberá milhões de requisições por dia e terá várias instâncias do back-end rodando em paralelo, onde você colocaria o load balancer na arquitetura e quais ferramentas ou tecnologias utilizaria para implementá-lo. Explique também a diferença entre usar um load balancer gerenciado na nuvem, como AWS ALB, e uma solução própria, como Enginix ou Haproxy.", 176_061, 198_821),
    ];
    let mut request = reported_request(&segments);
    let second_half = "você colocaria o load balancer na arquitetura e quais ferramentas ou tecnologias utilizaria para implementá-lo. Explique também a diferença entre usar um load balancer gerenciado na nuvem, como AWS LB, e uma solução própria, como Enginix ou HAPROXY.";
    request.prior_questions = vec![crate::domain::suggestion_gate::PriorQuestion {
        title: second_half.chars().take(180).collect(),
        block_id: "blk_load_balancer".to_string(),
        answered_at_ms: 1,
    }];
    let outcome = classifier.classify(&request).expect("classificação");
    assert_eq!(outcome.cue, SuggestionCue::AlreadyAnswered);
    assert!(!outcome.should_suggest);
    assert_eq!(outcome.backend, "heuristic");
}

/// A pergunta do amigo/dinheiro voltou só como cauda; a contenção lexical
/// reconhece o fragmento e não deixa o Laya sugerir de novo (ISS-081).
#[test]
#[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
fn regression_5f8595a7_friend_money_tail_is_already_answered() {
    let Some(directory) = model_dir() else {
        return;
    };
    let classifier = classifier(&directory);
    let segments = [
        ("t17", Speaker::They, "Vamos ver o que é", 346_110, 371_110),
        (
            "me26",
            Speaker::Me,
            "Me presta aqui, eu...",
            353_521,
            355_441,
        ),
        ("me27", Speaker::Me, "Tchau.", 355_441, 355_941),
        (
            "t18",
            Speaker::They,
            "Estar e conversa com ele com sinceridade. C. Ignora e muda de assunto.",
            371_110,
            375_710,
        ),
    ];
    let mut request = reported_request(&segments);
    let full = "Seguinte, um amigo te pede dinheiro emprestado toda semana e nunca devolve. O que você faz? A, continue emprestando porque é seu amigo. B, para de emprestar e conversa com ele com sinceridade. C, ignora e muda de assunto.";
    request.prior_questions = vec![crate::domain::suggestion_gate::PriorQuestion {
        title: full.to_string(),
        block_id: "blk_friend_money".to_string(),
        answered_at_ms: 1,
    }];
    let outcome = classifier.classify(&request).expect("classificação");
    assert_eq!(outcome.cue, SuggestionCue::AlreadyAnswered);
    assert!(!outcome.should_suggest);
    assert_eq!(outcome.backend, "heuristic");
}

/// A mesma pergunta de arquitetura, respondida e repetida, deve virar
/// `already_answered` pela guarda **lexical/Jaccard** (independente de idioma).
#[test]
#[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
fn regression_repeated_architecture_question_is_already_answered() {
    let Some(directory) = model_dir() else {
        return;
    };
    let classifier = classifier(&directory);
    let first = "ME: Muito bem, vamos começar então.\nTHEY: Pergunta de certativa. Arquitetura de software. Imagine que você é o tech lead de uma aplicação financeira que processa transferências bancárias. Dois clientes tentam transferir dinheiro da mesma conta ao mesmo tempo. E o sistema não pode permitir que o saldo fique negativo, nem que uma transferência seja processada.\nTHEY: como você projetaria essa...\nTHEY: a solução para garantir consistência dos dados e evitar problemas de concorrência. Responda como se estivesse em uma entrevista técnica. Depois, vou avaliar sua resposta e sugerir melhorias.";
    let prior_title = crate::domain::suggestion_gate::question_title(&conversation(first, "blk_1"));
    let prior = vec![crate::domain::suggestion_gate::PriorQuestion {
        title: prior_title,
        block_id: "blk_1".to_string(),
        answered_at_ms: 1,
    }];
    let repeated = format!("{first}\nME: eu usaria uma transação com isolamento serializável e idempotência.\nTHEY: Pergunta de certativa. Arquitetura de software. Imagine que você é o tech lead de uma aplicação financeira que processa transferências bancárias. Dois clientes tentam transferir dinheiro da mesma conta ao mesmo tempo. E o sistema não pode permitir que o saldo fique negativo, nem que uma transferência seja processada.");
    let outcome = classifier
        .classify(&request_with_prior(&repeated, "blk_2", prior))
        .expect("classificação");
    assert_eq!(outcome.cue, SuggestionCue::AlreadyAnswered);
    assert!(!outcome.should_suggest);
}

#[test]
#[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
fn laya_integration_marks_a_repeated_mutex_question_as_already_answered() {
    let Some(directory) = model_dir() else {
        return;
    };
    let classifier = classifier(&directory);
    let first_state = "ME: Não, né? Porque cada instância ia ter seu próprio mutex, então sem coordenar com as outras as 3 ainda poderiam modificar o recurso cada uma na sua.\nTHEY: Você mencionou o Mutex para controlar o acesso concorrente a um recurso. Mas me diga, o Mutex é a mesma coisa que uma fila.\nTHEY: Quais são as diferenças entre eles e em quais situações você utilizaria cada um?";
    let second_state = "THEY: Você mencionou o mutex para controlar o acesso concorrente a um recurso. Mas me diga, o mutex é a mesma coisa que o mutex.\nME: Não, ele é um bloqueio, né? Ele só garante o processo por vez, ele não é uma fila.\nTHEY: uma fila. Quais são as diferenças entre eles e em quais situações você utilizaria cada um?";

    let first_block = conversation(first_state, "blk_mutex_1");
    let first_outcome = classifier
        .classify(&request_with_prior(first_state, "blk_mutex_1", vec![]))
        .expect("classificação inicial");
    println!(
        "primeira pergunta: cue={} backend={} conf={:.2}",
        first_outcome.cue.as_str(),
        first_outcome.backend,
        first_outcome.confidence
    );
    let title = crate::domain::suggestion_gate::question_title(&first_block);
    let prior = vec![crate::domain::suggestion_gate::PriorQuestion {
        title,
        block_id: first_block.id.clone(),
        answered_at_ms: 10_000,
    }];
    let outcome = classifier
        .classify(&request_with_prior(second_state, "blk_mutex_2", prior))
        .expect("classificação");
    println!(
        "segunda pergunta: cue={} backend={} conf={:.2}",
        outcome.cue.as_str(),
        outcome.backend,
        outcome.confidence
    );
    assert_eq!(outcome.cue, SuggestionCue::AlreadyAnswered);
    assert!(!outcome.should_suggest);
}
