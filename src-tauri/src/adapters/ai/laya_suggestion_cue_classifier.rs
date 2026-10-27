//! Classificador Laya (ONNX) — BR-098.
//!
//! O SDK oficial é o `laya-onnx` (importado como `laya`) e o checkpoint alvo é
//! `convaiinnovations/laya-multilingual`. O artefato ONNX e o crate ainda não
//! estão publicados; este adaptador entrega o contrato completo e o mapeamento
//! testável (`choice` + confiança + abstenção). O binding ao engine entra em
//! [`LayaPredictor`] quando o artefato existir — nada de Python em runtime.
//!
//! Decisão: **uma** pergunta `choice` chamada `cue` com 5 categorias. O `noul`
//! não é usado (o model card documenta viés de rótulo). A decisão final é
//! `cue` + `answer_confidence` + abstenção; a taxonomia nova removeu
//! silêncio/hesitação e separa pergunta, comentário, já respondida e sem ação.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, sync_channel, Receiver, SyncSender, TrySendError};
use std::time::{Duration, Instant};

use crate::domain::suggestion_gate::{ConversationBlock, SuggestionCue};
use crate::domain::transcription::Speaker;
use crate::ports::suggestion_cue_classifier::{
    ClassificationOutcome, ClassificationRequest, ClassifierError, SuggestionCueClassifier,
};

/// Nome da pergunta no `QuestionSet`.
pub const CUE_QUESTION: &str = "cue";

/// Pergunta enviada ao modelo (o prompt real vem do SDK/tokenizer).
pub const CUE_QUESTION_TEXT: &str = "Você é um copiloto de reunião. Leia o contexto e o bloco mais recente. Classifique o bloco em exatamente uma opção: question (THEY fez uma pergunta que ME deve responder), comment (THEY fez um ponto/opinião e convida ME a comentar), already_answered (a pergunta atual já foi respondida no histórico recente ou pelo próprio ME neste bloco), no_action (conversa casual, alguém explicando algo, cumprimento ou fala sem pergunta/convite) ou insufficient_context (texto curto demais para decidir). Não existe mais classificação de silêncio ou hesitação.";

/// As 5 categorias, na ordem do `Question::choice`.
pub const CUE_OPTIONS: [(&str, SuggestionCue); 5] = [
    ("question", SuggestionCue::Question),
    ("comment", SuggestionCue::Comment),
    ("already_answered", SuggestionCue::AlreadyAnswered),
    ("no_action", SuggestionCue::NoAction),
    ("insufficient_context", SuggestionCue::InsufficientContext),
];

/// Descrição de cada opção (`Question::choice` usa label + descrição).
///
/// O head do Laya pontua um marcador por opção; a descrição ajuda o modelo a
/// separar `hesitation` de `lost_thread`, `silence` de `no_suggestion`, etc.
pub fn cue_option_description(label: &str) -> &'static str {
    match label {
        "question" => {
            "THEY asked ME a direct question or requested an explanation that ME has not answered yet."
        }
        "comment" => {
            "THEY shared a point, opinion, or proposal and invited ME to comment, agree, or add."
        }
        "already_answered" => {
            "The current question repeats a question already answered in the recent history, or ME already answered it in this block."
        }
        "no_action" => {
            "Casual conversation, small talk, or someone explaining something; there is no question or comment opportunity for ME."
        }
        "insufficient_context" => "Too little context to decide.",
        _ => "",
    }
}

/// Teto do estado enviado ao modelo (o checkpoint tem 1024 tokens).
const MAX_STATE_CHARS: usize = 6_000;
/// Capacidade da fila do worker (classificações em voo).
const QUEUE_CAPACITY: usize = 4;

/// Entrada do engine (o que o binding ONNX consome).
#[derive(Debug, Clone, PartialEq)]
pub struct LayaInput {
    /// Blocos anteriores + bloco atual, em texto.
    pub state: String,
    pub question: String,
    pub options: Vec<String>,
    /// Limiar pedido ao `PredictOptions::min_confidence`.
    pub min_confidence: f32,
}

/// Resposta crua do engine (o que o SDK devolve).
#[derive(Debug, Clone, PartialEq)]
pub struct LayaPrediction {
    /// `answer.as_choice()`.
    pub choice: Option<String>,
    /// `answer.answer_confidence()`.
    pub confidence: f32,
    /// `answer.abstention() == Some("abstained")`.
    pub abstained: bool,
    /// `answer.low_confidence()`.
    pub low_confidence: bool,
}

/// Engine do Laya.
///
/// Implementação real: `LayaEngine` do crate `laya-onnx`, dono da thread. Nos
/// testes, um fake determinístico.
pub trait LayaPredictor: Send + Sync {
    fn predict(&self, input: &LayaInput) -> Result<LayaPrediction, ClassifierError>;
}

/// Configuração do adaptador.
#[derive(Debug, Clone, PartialEq)]
pub struct LayaClassifierConfig {
    /// Limiar de confiança do preset de sensibilidade.
    pub threshold: f32,
    /// Timeout do worker (o scheduler nunca fica preso).
    pub timeout_ms: u64,
    /// `multilingual` / `english` (só rótulo/diagnóstico).
    pub model: String,
}

impl Default for LayaClassifierConfig {
    fn default() -> Self {
        Self {
            threshold: 0.70,
            timeout_ms: 12_000,
            model: "multilingual".to_string(),
        }
    }
}

impl LayaClassifierConfig {
    /// Traduz as settings para a configuração do adaptador.
    ///
    /// O limiar segue o preset de sensibilidade (o gate também aplica o valor
    /// vivo); o timeout limita a espera do worker e o checkpoint só rotula o
    /// backend.
    pub fn from_settings(settings: &crate::domain::settings::Settings) -> Self {
        let sensitivity = crate::domain::suggestion_gate::ClassifierSensitivity::from_str(
            &settings.suggestion_classifier_sensitivity,
        )
        .unwrap_or_default();
        Self {
            threshold: sensitivity.laya_threshold(),
            timeout_ms: u64::from(settings.suggestion_classifier_timeout_ms),
            model: settings.suggestion_classifier_checkpoint.clone(),
        }
    }
}

/// Decisão derivada da resposta do Laya.
#[derive(Debug, Clone, PartialEq)]
pub struct LayaDecision {
    pub cue: SuggestionCue,
    pub should_suggest: bool,
    pub abstained: bool,
}

/// Mapeia a `choice` para a cue de domínio (tolerante a caixa/espaços).
pub fn cue_from_choice(choice: &str) -> Option<SuggestionCue> {
    let normalized = choice.trim().to_lowercase();
    CUE_OPTIONS
        .iter()
        .find(|(label, _)| *label == normalized)
        .map(|(_, cue)| *cue)
}

/// Deriva `cue`/`should_suggest`/abstenção **sem** confiar no backend.
///
/// Abstenção e baixa confiança viram `insufficient_context`; `no_suggestion`
/// nunca gera sugestão; a confiança precisa vencer o limiar.
pub fn decide(prediction: &LayaPrediction, threshold: f32) -> LayaDecision {
    if prediction.abstained || prediction.low_confidence {
        return LayaDecision {
            cue: SuggestionCue::InsufficientContext,
            should_suggest: false,
            abstained: true,
        };
    }

    let cue = prediction
        .choice
        .as_deref()
        .and_then(cue_from_choice)
        .unwrap_or(SuggestionCue::InsufficientContext);

    if cue == SuggestionCue::InsufficientContext {
        return LayaDecision {
            cue,
            should_suggest: false,
            abstained: true,
        };
    }

    let confidence_ok = prediction.confidence.is_finite() && prediction.confidence >= threshold;
    LayaDecision {
        cue,
        should_suggest: cue.can_suggest() && confidence_ok,
        abstained: false,
    }
}

fn speaker_label(speaker: Speaker) -> &'static str {
    match speaker {
        Speaker::Me => "ME",
        Speaker::They => "THEY",
    }
}

fn render_block(block: &ConversationBlock) -> String {
    block
        .turns
        .iter()
        .filter_map(|turn| {
            let text = turn.text.trim();
            if text.is_empty() {
                None
            } else {
                Some(format!("{}: {text}", speaker_label(turn.speaker)))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Converte blocos em estado textual para o modelo.
///
/// Inclui os blocos de contexto (mais antigos primeiro) e o bloco atual. Se o
/// texto passar do teto, mantém a parte **mais recente** (o fim da conversa é o
/// que decide a cue).
pub fn render_state(context: &[ConversationBlock], block: &ConversationBlock) -> String {
    let mut parts: Vec<String> = context.iter().map(render_block).collect();
    parts.push(render_block(block));
    let state = parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n---\n");

    if state.chars().count() <= MAX_STATE_CHARS {
        return state;
    }
    let truncated: String = state
        .chars()
        .rev()
        .take(MAX_STATE_CHARS.saturating_sub(1))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("…{truncated}")
}

fn focus_for_cue(cue: SuggestionCue) -> Option<String> {
    let text = match cue {
        SuggestionCue::Question => "responder à pergunta",
        SuggestionCue::Comment => "comentar o ponto",
        SuggestionCue::AlreadyAnswered
        | SuggestionCue::NoAction
        | SuggestionCue::InsufficientContext => return None,
    };
    Some(text.to_string())
}

fn evidence_for(block: &ConversationBlock, cue: SuggestionCue) -> String {
    let preferred = match cue {
        SuggestionCue::Question | SuggestionCue::Comment | SuggestionCue::AlreadyAnswered => {
            Some(Speaker::They)
        }
        SuggestionCue::NoAction | SuggestionCue::InsufficientContext => None,
    };
    let turn = preferred
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

/// Resolve o diretório do modelo: override de dev **ou** versão ativa.
///
/// Ausência de diretório/`model.onnx` vira `Unavailable` para a UI mostrar o
/// erro e o heurístico continuar disponível.
pub fn resolve_model_dir(
    override_path: &str,
    active: Option<PathBuf>,
    bundled: Option<PathBuf>,
) -> Result<PathBuf, ClassifierError> {
    let candidate = if !override_path.trim().is_empty() {
        PathBuf::from(override_path.trim())
    } else if let Some(active) = active {
        active
    } else if let Some(bundled) = bundled {
        bundled
    } else {
        return Err(ClassifierError::Unavailable);
    };
    if !candidate.is_dir() || !has_model_file(&candidate) {
        return Err(ClassifierError::Unavailable);
    }
    Ok(candidate)
}

fn has_model_file(dir: &Path) -> bool {
    dir.join("model.onnx").is_file()
}

/// Job enviado para o worker dono do engine.
struct Job {
    input: LayaInput,
    reply: mpsc::Sender<Result<LayaPrediction, ClassifierError>>,
}

/// Adaptador do classificador Laya.
///
/// O engine fica numa thread própria (carregado uma vez); `classify` envia o
/// job e espera com timeout. Se o worker travar, o scheduler recebe `Timeout`
/// e segue — nunca fica preso.
pub struct LayaSuggestionCueClassifier {
    sender: SyncSender<Job>,
    config: LayaClassifierConfig,
}

impl LayaSuggestionCueClassifier {
    pub fn new(predictor: Box<dyn LayaPredictor>, config: LayaClassifierConfig) -> Self {
        let (sender, receiver): (SyncSender<Job>, Receiver<Job>) = sync_channel(QUEUE_CAPACITY);
        std::thread::spawn(move || {
            while let Ok(job) = receiver.recv() {
                let result = predictor.predict(&job.input);
                let _ = job.reply.send(result);
            }
        });
        Self { sender, config }
    }

    /// Aquece o worker/engine em background (não bloqueia o boot).
    ///
    /// A primeira inferência real não paga a leitura de ~1,2 GB do ONNX; se a
    /// carga falhar, o `failed` do predictor evita tentar de novo a cada bloco.
    pub fn warm_up(&self) {
        let (reply, _response) = mpsc::channel();
        let input = LayaInput {
            state: "ME: hello\nTHEY: hi".to_string(),
            question: CUE_QUESTION_TEXT.to_string(),
            options: CUE_OPTIONS
                .iter()
                .map(|(label, _)| (*label).to_string())
                .collect(),
            min_confidence: 0.0,
        };
        let _ = self.sender.try_send(Job { input, reply });
    }

    fn input(&self, request: &ClassificationRequest) -> LayaInput {
        let mut question = CUE_QUESTION_TEXT.to_string();
        if !request.mode_context.trim().is_empty() {
            let context: String = request.mode_context.chars().take(800).collect();
            question.push_str("\n\n[Agente selecionado]\n");
            question.push_str(&context);
        }
        if !request.prior_questions.is_empty() {
            question.push_str("\n\n[Perguntas já respondidas recentemente]\n");
            for (index, prior) in request.prior_questions.iter().enumerate() {
                question.push_str(&format!("{}. {}\n", index + 1, prior.title.trim()));
            }
            question.push_str(
                "Se o bloco atual repetir uma dessas perguntas, escolha already_answered.",
            );
        }
        LayaInput {
            state: render_state(&request.context, &request.block),
            question,
            options: CUE_OPTIONS
                .iter()
                .map(|(label, _)| (*label).to_string())
                .collect(),
            min_confidence: self.config.threshold,
        }
    }

    fn outcome(
        &self,
        request: &ClassificationRequest,
        prediction: &LayaPrediction,
        latency_ms: u64,
    ) -> ClassificationOutcome {
        let decision = decide(prediction, self.config.threshold);
        ClassificationOutcome {
            block_id: request.block.id.clone(),
            session_id: request.block.session_id.clone(),
            cue: decision.cue,
            should_suggest: decision.should_suggest,
            confidence: prediction.confidence,
            evidence: evidence_for(&request.block, decision.cue),
            focus: focus_for_cue(decision.cue),
            backend: "laya".to_string(),
            model: if self.config.model.is_empty() {
                None
            } else {
                Some(self.config.model.clone())
            },
            latency_ms,
            block_start_ms: request.block.start_ms,
            block_end_ms: request.block.end_ms,
            turn_count: request.block.turns.len(),
            classified_at_ms: request.now_ms,
            abstained: decision.abstained,
        }
        .normalized()
    }
}

impl SuggestionCueClassifier for LayaSuggestionCueClassifier {
    fn classify(
        &self,
        request: &ClassificationRequest,
    ) -> std::result::Result<ClassificationOutcome, ClassifierError> {
        let started = Instant::now();
        let (reply, response) = mpsc::channel();
        self.sender
            .try_send(Job {
                input: self.input(request),
                reply,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) | TrySendError::Disconnected(_) => {
                    ClassifierError::Unavailable
                }
            })?;

        let prediction = response
            .recv_timeout(Duration::from_millis(self.config.timeout_ms))
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => {
                    tracing::warn!(
                        "Laya timeout após {} ms (modelo pode estar carregando)",
                        self.config.timeout_ms
                    );
                    ClassifierError::Timeout
                }
                mpsc::RecvTimeoutError::Disconnected => {
                    tracing::warn!("worker do Laya desconectado");
                    ClassifierError::Unavailable
                }
            })??;

        Ok(self.outcome(request, &prediction, started.elapsed().as_millis() as u64))
    }

    fn backend_id(&self) -> &'static str {
        "laya"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use crate::domain::suggestion_gate::BlockTurn;

    fn block(turns: Vec<(Speaker, &str)>) -> ConversationBlock {
        ConversationBlock {
            id: "blk_1".to_string(),
            session_id: "s1".to_string(),
            start_ms: 0,
            end_ms: 2_000,
            turns: turns
                .into_iter()
                .map(|(speaker, text)| BlockTurn {
                    speaker,
                    text: text.to_string(),
                    start_ms: 0,
                    end_ms: 1_000,
                })
                .collect(),
            me_chars: 10,
            they_chars: 10,
            is_open: false,
        }
    }

    fn request(block: ConversationBlock) -> ClassificationRequest {
        ClassificationRequest {
            session_id: "s1".to_string(),
            block,
            context: vec![],
            locale: "pt".to_string(),
            mode_id: "default".to_string(),
            mode_context: String::new(),
            allow_single_speaker: true,
            prior_questions: vec![],
            now_ms: 10_000,
        }
    }

    struct FakePredictor {
        prediction: LayaPrediction,
        delay_ms: u64,
        calls: Arc<AtomicUsize>,
    }

    impl FakePredictor {
        fn new(prediction: LayaPrediction) -> Self {
            Self {
                prediction,
                delay_ms: 0,
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn slow(prediction: LayaPrediction, delay_ms: u64) -> Self {
            Self {
                prediction,
                delay_ms,
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl LayaPredictor for FakePredictor {
        fn predict(&self, _input: &LayaInput) -> Result<LayaPrediction, ClassifierError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(self.delay_ms));
            }
            Ok(self.prediction.clone())
        }
    }

    fn prediction(choice: &str, confidence: f32) -> LayaPrediction {
        LayaPrediction {
            choice: Some(choice.to_string()),
            confidence,
            abstained: false,
            low_confidence: false,
        }
    }

    #[test]
    fn laya_question_includes_the_agent_context() {
        let classifier = LayaSuggestionCueClassifier::new(
            Box::new(FakePredictor::new(prediction("question", 0.9))),
            LayaClassifierConfig::default(),
        );
        let mut request = request(block(vec![
            (Speaker::Me, "eu explico"),
            (Speaker::They, "pode detalhar?"),
        ]));
        request.mode_context = "Agente: Entrevista Técnica (engineering)".to_string();
        let input = classifier.input(&request);
        assert!(input.question.contains("Entrevista Técnica"));
        assert!(input.question.contains(CUE_QUESTION_TEXT));
    }

    #[test]
    fn laya_config_uses_the_calibrated_sensitivity_threshold() {
        use crate::domain::settings::Settings;

        let settings = Settings {
            suggestion_classifier_sensitivity: "balanced".to_string(),
            ..Settings::default()
        };
        assert_eq!(
            LayaClassifierConfig::from_settings(&settings).threshold,
            0.50
        );
        let high = Settings {
            suggestion_classifier_sensitivity: "high".to_string(),
            ..settings
        };
        assert_eq!(LayaClassifierConfig::from_settings(&high).threshold, 0.35);
    }

    #[test]
    fn laya_maps_choice_to_cue_and_confidence() {
        let classifier = LayaSuggestionCueClassifier::new(
            Box::new(FakePredictor::new(prediction("question", 0.91))),
            LayaClassifierConfig::default(),
        );
        let outcome = classifier
            .classify(&request(block(vec![
                (Speaker::Me, "eu explico"),
                (Speaker::They, "pode detalhar?"),
            ])))
            .expect("classificação");

        assert_eq!(outcome.cue, SuggestionCue::Question);
        assert!(outcome.should_suggest);
        assert_eq!(outcome.backend, "laya");
        assert_eq!(outcome.model.as_deref(), Some("multilingual"));
        assert_eq!(outcome.confidence, 0.91);
        assert_eq!(outcome.evidence, "pode detalhar?");
        assert_eq!(classifier.backend_id(), "laya");
    }

    #[test]
    fn laya_abstention_maps_to_insufficient_context() {
        let mut abstained = prediction("question", 0.9);
        abstained.abstained = true;
        let classifier = LayaSuggestionCueClassifier::new(
            Box::new(FakePredictor::new(abstained)),
            LayaClassifierConfig::default(),
        );
        let outcome = classifier
            .classify(&request(block(vec![(Speaker::Me, "oi")])))
            .expect("classificação");

        assert_eq!(outcome.cue, SuggestionCue::InsufficientContext);
        assert!(outcome.abstained);
        assert!(!outcome.should_suggest);
        assert!(outcome.focus.is_none());
    }

    #[test]
    fn laya_low_confidence_abstains() {
        let mut low = prediction("comment", 0.3);
        low.low_confidence = true;
        let classifier = LayaSuggestionCueClassifier::new(
            Box::new(FakePredictor::new(low)),
            LayaClassifierConfig::default(),
        );
        let outcome = classifier
            .classify(&request(block(vec![(Speaker::They, "e você?")])))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::InsufficientContext);
        assert!(outcome.abstained);
        assert!(!outcome.should_suggest);
    }

    #[test]
    fn laya_should_suggest_is_false_for_no_action() {
        let classifier = LayaSuggestionCueClassifier::new(
            Box::new(FakePredictor::new(prediction("no_action", 0.99))),
            LayaClassifierConfig::default(),
        );
        let outcome = classifier
            .classify(&request(block(vec![(Speaker::Me, "combinado")])))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::NoAction);
        assert!(!outcome.should_suggest);
        assert!(!outcome.abstained);
        assert!(outcome.focus.is_none());
    }

    #[test]
    fn laya_below_threshold_does_not_suggest() {
        let classifier = LayaSuggestionCueClassifier::new(
            Box::new(FakePredictor::new(prediction("question", 0.55))),
            LayaClassifierConfig {
                threshold: 0.70,
                ..LayaClassifierConfig::default()
            },
        );
        let outcome = classifier
            .classify(&request(block(vec![(Speaker::They, "pode explicar?")])))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::Question);
        assert!(!outcome.should_suggest);
    }

    #[test]
    fn laya_unknown_choice_maps_to_insufficient_context() {
        let classifier = LayaSuggestionCueClassifier::new(
            Box::new(FakePredictor::new(prediction("talvez", 0.9))),
            LayaClassifierConfig::default(),
        );
        let outcome = classifier
            .classify(&request(block(vec![(Speaker::Me, "hmm")])))
            .expect("classificação");
        assert_eq!(outcome.cue, SuggestionCue::InsufficientContext);
        assert!(outcome.abstained);
    }

    #[test]
    fn laya_timeout_returns_timeout_without_blocking() {
        let classifier = LayaSuggestionCueClassifier::new(
            Box::new(FakePredictor::slow(prediction("question", 0.9), 500)),
            LayaClassifierConfig {
                timeout_ms: 40,
                ..LayaClassifierConfig::default()
            },
        );
        let started = Instant::now();
        let error = classifier
            .classify(&request(block(vec![(Speaker::They, "pode explicar?")])))
            .expect_err("timeout");
        assert!(matches!(error, ClassifierError::Timeout));
        assert!(
            started.elapsed() < Duration::from_millis(300),
            "não esperou o worker terminar"
        );
    }

    #[test]
    fn laya_missing_model_is_unavailable() {
        assert!(matches!(
            resolve_model_dir("", None, None),
            Err(ClassifierError::Unavailable)
        ));

        let temp = tempfile::tempdir().expect("tempdir");
        assert!(matches!(
            resolve_model_dir(temp.path().to_str().unwrap(), None, None),
            Err(ClassifierError::Unavailable)
        ));

        std::fs::write(temp.path().join("model.onnx"), b"onnx").unwrap();
        assert!(resolve_model_dir("", Some(temp.path().to_path_buf()), None).is_ok());
        assert!(resolve_model_dir(temp.path().to_str().unwrap(), None, None).is_ok());
    }

    #[test]
    fn laya_dev_override_wins_over_the_active_model() {
        let override_dir = tempfile::tempdir().expect("override");
        std::fs::write(override_dir.path().join("model.onnx"), b"onnx").unwrap();
        let active_dir = tempfile::tempdir().expect("active");
        std::fs::write(active_dir.path().join("model.onnx"), b"onnx").unwrap();

        let resolved = resolve_model_dir(
            override_dir.path().to_str().unwrap(),
            Some(active_dir.path().to_path_buf()),
            None,
        )
        .expect("resolve");
        assert_eq!(resolved, override_dir.path());
    }

    #[test]
    fn laya_bundled_model_is_the_last_fallback() {
        let bundled_dir = tempfile::tempdir().expect("bundled");
        std::fs::write(bundled_dir.path().join("model.onnx"), b"onnx").unwrap();
        let active_dir = tempfile::tempdir().expect("active");
        std::fs::write(active_dir.path().join("model.onnx"), b"onnx").unwrap();

        // Active (instalado pelo usuário) vence o bundled.
        let resolved = resolve_model_dir(
            "",
            Some(active_dir.path().to_path_buf()),
            Some(bundled_dir.path().to_path_buf()),
        )
        .expect("resolve");
        assert_eq!(resolved, active_dir.path());

        // Sem modelo instalado, usa o bundled.
        let resolved =
            resolve_model_dir("", None, Some(bundled_dir.path().to_path_buf())).expect("bundled");
        assert_eq!(resolved, bundled_dir.path());
    }

    struct FailingPredictor;

    impl LayaPredictor for FailingPredictor {
        fn predict(&self, _input: &LayaInput) -> Result<LayaPrediction, ClassifierError> {
            Err(ClassifierError::InvalidResponse)
        }
    }

    #[test]
    fn laya_does_not_log_block_text() {
        // O erro do backend não carrega o texto do bloco (só a UI vê a
        // evidência curta, por contrato do `ClassificationOutcome`).
        let classifier = LayaSuggestionCueClassifier::new(
            Box::new(FailingPredictor),
            LayaClassifierConfig::default(),
        );
        let error = classifier
            .classify(&request(block(vec![
                (Speaker::Me, "segredo do cliente"),
                (Speaker::They, "pode repetir?"),
            ])))
            .expect_err("falha");
        assert!(!error.to_string().contains("segredo do cliente"));
    }

    #[test]
    fn render_state_keeps_both_speakers_and_orders_context_first() {
        let context = block(vec![(Speaker::Me, "contexto antigo")]);
        let current = block(vec![
            (Speaker::Me, "minha fala"),
            (Speaker::They, "fala do outro"),
        ]);
        let state = render_state(&[context], &current);
        assert!(state.starts_with("ME: contexto antigo"));
        assert!(state.contains("---"));
        assert!(state.contains("THEY: fala do outro"));
        assert!(state.find("minha fala") < state.find("fala do outro"));
    }

    #[test]
    fn render_state_caps_the_size_keeping_the_tail() {
        let long = "x".repeat(MAX_STATE_CHARS * 2);
        let state = render_state(&[], &block(vec![(Speaker::Me, &long)]));
        assert!(state.chars().count() <= MAX_STATE_CHARS);
        assert!(state.starts_with('…'));
    }

    #[test]
    fn cue_mapping_accepts_case_and_spaces() {
        assert_eq!(
            cue_from_choice("  QUESTION "),
            Some(SuggestionCue::Question)
        );
        assert_eq!(cue_from_choice("comment"), Some(SuggestionCue::Comment));
        assert_eq!(
            cue_from_choice("already_answered"),
            Some(SuggestionCue::AlreadyAnswered)
        );
        assert_eq!(cue_from_choice("noul"), None);
    }

    /// Bloco rotulado do POC (`poc/laya-classifier/fixtures/blocks.jsonl`).
    #[derive(serde::Deserialize)]
    struct FixtureBlock {
        id: String,
        #[allow(dead_code)]
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

    fn fixtures() -> Vec<FixtureBlock> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../poc/laya-classifier/fixtures/blocks.jsonl");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("fixtures em {}: {error}", path.display()));
        raw.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("bloco do fixture"))
            .collect()
    }

    fn turns_from_state(state: &str) -> Vec<BlockTurn> {
        let mut turns = Vec::new();
        for (index, line) in state.lines().enumerate() {
            let Some((speaker, text)) = line.split_once(": ") else {
                continue;
            };
            let speaker = match speaker.trim() {
                "ME" => Speaker::Me,
                "THEY" => Speaker::They,
                _ => continue,
            };
            let start = index as i64 * 1_000;
            turns.push(BlockTurn {
                speaker,
                text: text.trim().to_string(),
                start_ms: start,
                end_ms: start + 900,
            });
        }
        turns
    }

    fn conversation_from(fixture: &FixtureBlock, id: &str) -> ConversationBlock {
        let turns = turns_from_state(&fixture.state);
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

    fn fixture_request(fixture: &FixtureBlock) -> ClassificationRequest {
        let block = conversation_from(fixture, &fixture.id);
        let context = if fixture.context.trim().is_empty() {
            vec![]
        } else {
            vec![conversation_from(
                &FixtureBlock {
                    id: "ctx".to_string(),
                    locale: fixture.locale.clone(),
                    state: fixture.context.clone(),
                    context: String::new(),
                    gap_ms: 0,
                    prior_questions: Vec::new(),
                    label_cue: "no_action".to_string(),
                    label_should_suggest: false,
                },
                "ctx",
            )]
        };
        ClassificationRequest {
            session_id: "s1".to_string(),
            block,
            context,
            locale: fixture.locale.clone(),
            mode_id: "default".to_string(),
            mode_context: String::new(),
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

    #[test]
    fn laya_eval_fixture_shape() {
        let fixtures = fixtures();
        assert!(fixtures.len() >= 20, "mínimo de 20 blocos");
        let mut counts = std::collections::HashMap::new();
        for fixture in &fixtures {
            let cue = cue_from_choice(&fixture.label_cue)
                .unwrap_or_else(|| panic!("cue desconhecida em {}", fixture.id));
            *counts.entry(cue).or_insert(0_usize) += 1;
            assert_eq!(
                fixture.label_should_suggest,
                cue.can_suggest(),
                "rótulo incoerente em {}",
                fixture.id
            );
            assert!(
                !fixture.state.trim().is_empty(),
                "estado vazio em {}",
                fixture.id
            );
        }
        assert_eq!(counts.len(), CUE_OPTIONS.len(), "todas as 7 cues");
        for (label, _) in CUE_OPTIONS {
            let count = counts
                .get(&cue_from_choice(label).expect("cue"))
                .copied()
                .unwrap_or(0);
            assert!(count >= 3, "cue {label} tem só {count} exemplos");
        }
    }

    /// Debug do POC: imprime choice/confiança brutos do modelo.
    ///
    /// ```bash
    /// LAYA_ONNX_DIR=... cargo test --lib debug_laya_raw --features laya -- --ignored --nocapture
    /// ```
    #[cfg(feature = "laya")]
    #[test]
    #[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
    fn debug_laya_raw_predictions() {
        use crate::adapters::ai::laya_onnx_predictor::OnnxLayaPredictor;

        let directory = std::env::var("LAYA_ONNX_DIR").expect("LAYA_ONNX_DIR");
        let predictor = OnnxLayaPredictor::new(directory).expect("modelo Laya");
        for fixture in fixtures() {
            let request = fixture_request(&fixture);
            let input = LayaInput {
                state: render_state(&request.context, &request.block),
                question: CUE_QUESTION_TEXT.to_string(),
                options: CUE_OPTIONS
                    .iter()
                    .map(|(label, _)| (*label).to_string())
                    .collect(),
                min_confidence: 0.0,
            };
            match predictor.predict(&input) {
                Ok(prediction) => println!(
                    "{} expected={} choice={:?} conf={:.3} abstained={} low={}",
                    fixture.id,
                    fixture.label_cue,
                    prediction.choice,
                    prediction.confidence,
                    prediction.abstained,
                    prediction.low_confidence
                ),
                Err(error) => println!("{} erro={error}", fixture.id),
            }
        }
    }

    /// Experimento: pergunta binária "deve sugerir?" (3 opções).
    #[cfg(feature = "laya")]
    #[test]
    #[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
    fn debug_laya_binary_should_suggest() {
        let directory = std::env::var("LAYA_ONNX_DIR").expect("LAYA_ONNX_DIR");
        let engine = laya::LayaEngine::create(laya::LayaOptions {
            model_directory: Some(std::path::PathBuf::from(directory)),
            allow_download: false,
            hugging_face_token: Some(String::new()),
            ..Default::default()
        })
        .expect("engine");

        let question = laya::Question::choice(
            "You are a meeting copilot. Read the latest conversation block. Should the copilot suggest a response now?",
            [
                ("suggest", "A useful suggestion can be made now."),
                ("do_not_suggest", "The conversation is flowing; no suggestion is needed."),
                ("insufficient_context", "There is not enough context to decide."),
            ],
        )
        .expect("question");
        let questions = laya::QuestionSet::new()
            .with("should_suggest", question)
            .expect("set");
        let options = laya::PredictOptions::new()
            .min_confidence(laya::MinConfidence::scalar(0.0).expect("min"));

        let mut correct = 0;
        let fixtures = fixtures();
        for fixture in &fixtures {
            let request = fixture_request(fixture);
            let state = render_state(&request.context, &request.block);
            let result = engine
                .predict_with_options(state, &questions, &options)
                .expect("predict");
            let answer = result.get("should_suggest").expect("answer");
            let choice = answer.as_choice().map(|c| c.choice.as_str()).unwrap_or("?");
            let predicted = choice == "suggest";
            if predicted == fixture.label_should_suggest {
                correct += 1;
            }
            println!(
                "{} expected_suggest={} choice={} conf={:.3}",
                fixture.id,
                fixture.label_should_suggest,
                choice,
                answer.answer_confidence()
            );
        }
        println!("binary accuracy: {correct}/{}", fixtures.len());
    }

    /// Experimento: `noul` "deve sugerir?" (o model card avisa de viés).
    #[cfg(feature = "laya")]
    #[test]
    #[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
    fn debug_laya_noul_should_suggest() {
        let directory = std::env::var("LAYA_ONNX_DIR").expect("LAYA_ONNX_DIR");
        let engine = laya::LayaEngine::create(laya::LayaOptions {
            model_directory: Some(std::path::PathBuf::from(directory)),
            allow_download: false,
            hugging_face_token: Some(String::new()),
            ..Default::default()
        })
        .expect("engine");
        let question = laya::Question::noul(
            "The meeting copilot should suggest a response now.",
            "do not suggest",
            "suggest",
        );
        let questions = laya::QuestionSet::new()
            .with("should_suggest", question)
            .expect("set");
        let options = laya::PredictOptions::new()
            .min_confidence(laya::MinConfidence::scalar(0.0).expect("min"));

        let mut correct = 0;
        let fixtures = fixtures();
        for fixture in &fixtures {
            let request = fixture_request(fixture);
            let state = render_state(&request.context, &request.block);
            let result = engine
                .predict_with_options(state, &questions, &options)
                .expect("predict");
            let answer = result.get("should_suggest").expect("answer");
            let probability = answer.as_noul().map(|n| n.probability).unwrap_or(0.0);
            let predicted = probability >= 0.5;
            if predicted == fixture.label_should_suggest {
                correct += 1;
            }
            println!(
                "{} expected={} p_true={:.3}",
                fixture.id, fixture.label_should_suggest, probability
            );
        }
        println!("noul accuracy: {correct}/{}", fixtures.len());
    }

    /// Eval real: exige `LAYA_ONNX_DIR` com o artefato exportado.
    ///
    /// ```bash
    /// LAYA_ONNX_DIR=~/models/laya/multilingual \
    ///   cargo test --lib eval_laya_on_fixtures --features laya -- --ignored --nocapture
    /// ```
    #[cfg(feature = "laya")]
    #[test]
    #[ignore = "exige LAYA_ONNX_DIR e o artefato ONNX exportado"]
    fn eval_laya_on_fixtures() {
        use crate::adapters::ai::laya_onnx_predictor::OnnxLayaPredictor;
        use crate::domain::classifier_eval::{evaluate, LabeledPrediction};

        let directory = std::env::var("LAYA_ONNX_DIR")
            .expect("defina LAYA_ONNX_DIR com o diretório do model.onnx");
        let predictor = OnnxLayaPredictor::new(directory).expect("modelo Laya");
        let classifier = LayaSuggestionCueClassifier::new(
            Box::new(predictor),
            LayaClassifierConfig {
                threshold: 0.50,
                timeout_ms: 10_000,
                model: "multilingual".to_string(),
            },
        );

        let fixtures = fixtures();
        let predictions: Vec<LabeledPrediction> = fixtures
            .iter()
            .map(|fixture| {
                let request = fixture_request(fixture);
                let outcome = classifier.classify(&request).expect("classifica");
                LabeledPrediction {
                    expected_cue: cue_from_choice(&fixture.label_cue).expect("cue"),
                    predicted_cue: outcome.cue,
                    expected_should_suggest: fixture.label_should_suggest,
                    predicted_should_suggest: outcome.should_suggest,
                    latency_ms: outcome.latency_ms,
                }
            })
            .collect();

        let report = evaluate(&predictions);
        println!("{}", serde_json::to_string_pretty(&report).expect("json"));
        println!("\n{}", report.to_markdown());
        assert_eq!(report.total, fixtures.len());
    }

    /// Baseline do gate com o heurístico (o Laya depende do artefato publicado).
    ///
    /// `cargo test --lib eval_heuristic_baseline_on_fixtures -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn eval_heuristic_baseline_on_fixtures() {
        use crate::adapters::ai::heuristic_suggestion_cue_classifier::HeuristicSuggestionCueClassifier;
        use crate::domain::classifier_eval::{evaluate, LabeledPrediction};

        let classifier = HeuristicSuggestionCueClassifier::default();
        let fixtures = fixtures();
        let predictions: Vec<LabeledPrediction> = fixtures
            .iter()
            .map(|fixture| {
                let request = fixture_request(fixture);
                let outcome = classifier.classify(&request).expect("classifica");
                LabeledPrediction {
                    expected_cue: cue_from_choice(&fixture.label_cue).expect("cue"),
                    predicted_cue: outcome.cue,
                    expected_should_suggest: fixture.label_should_suggest,
                    predicted_should_suggest: outcome.should_suggest,
                    latency_ms: outcome.latency_ms,
                }
            })
            .collect();

        let report = evaluate(&predictions);
        println!("{}", serde_json::to_string_pretty(&report).expect("json"));
        println!("\n{}", report.to_markdown());
        assert_eq!(report.total, fixtures.len());
    }
}
