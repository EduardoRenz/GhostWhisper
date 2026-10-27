//! Caso de uso do gate por blocos (BR-093/BR-094).
//!
//! Monta os blocos a partir das falas finais, chama o classificador, guarda a
//! última classificação por sessão e aplica as guardas de geração. O scheduler
//! (task 10) é o driver: ele chama `classify_latest` e, se o gate permitir,
//! chama o `SuggestionGenerator`.
//!
//! TODO(task 10): remover o `allow(dead_code)` quando o scheduler ligar o gate.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use crate::domain::settings::Settings;
use crate::domain::suggestion_gate::{
    build_blocks, decide, question_title, they_target, BlockConfig, ClassifierSensitivity,
    ConversationBlock, GateDecision, GateSkipReason, GateTick, HeuristicConfig, PriorQuestion,
    SuggestionCue,
};
use crate::ports::clock::Clock;
use crate::ports::event_publisher::{
    EventPublisher, SuggestionClassificationEvent, SuggestionClassifierState,
};
use crate::ports::repositories::{SessionRepository, TranscriptionsRepository};
use crate::ports::suggestion_cue_classifier::{
    ClassificationOutcome, ClassificationRequest, SuggestionCueClassifier,
};
use crate::ports::suggestion_gate::{
    GateResult, SuggestionGate, SuggestionGateConfig, SuggestionGateError,
};

/// Contexto curto do agente selecionado para o classificador.
///
/// O gerador já usa o prompt completo; o classificador recebe nome, categoria,
/// contexto fixo e um recorte do foco (sem gastar o orçamento do modelo).
fn selected_mode_context(settings: &Settings) -> String {
    let Some(mode) = crate::application::prompts::assistant::resolve_mode(
        &settings.assistant_mode_id,
        &settings.custom_assistant_modes,
    )
    .or_else(|| {
        crate::application::prompts::assistant::factory_modes()
            .into_iter()
            .find(|mode| mode.id == settings.assistant_mode_id)
    }) else {
        return String::new();
    };
    let mut parts = vec![format!("Agente: {} ({})", mode.name, mode.category)];
    if !mode.context.trim().is_empty() {
        parts.push(format!("Contexto do agente: {}", mode.context.trim()));
    }
    if !mode.suggestion_prompt.trim().is_empty() {
        let focus: String = mode.suggestion_prompt.chars().take(500).collect();
        parts.push(format!("Foco do agente: {focus}"));
    }
    parts.join("\n")
}

/// Traduz as settings para a configuração pura do gate (task 04).
pub fn config_from_settings(settings: &Settings) -> SuggestionGateConfig {
    let sensitivity = ClassifierSensitivity::from_str(&settings.suggestion_classifier_sensitivity)
        .unwrap_or_default();
    let allowed_cues: Vec<SuggestionCue> = settings
        .suggestion_allowed_cues
        .iter()
        .filter_map(|value| SuggestionCue::from_str(value))
        .filter(|cue| cue.can_suggest())
        .collect();

    SuggestionGateConfig {
        enabled: settings.suggestion_auto_refresh,
        block: BlockConfig {
            gap_ms: i64::from(settings.suggestion_block_gap_ms),
            max_block_ms: i64::from(settings.suggestion_block_max_ms),
            max_turns: settings.suggestion_block_max_turns as usize,
            min_chars: 8,
            context_blocks: settings.suggestion_context_blocks as usize,
        },
        heuristic: HeuristicConfig::default(),
        confidence_min: if settings.suggestion_classifier_backend == "laya" {
            sensitivity.laya_threshold()
        } else {
            sensitivity.threshold()
        },
        cooldown_ms: i64::from(settings.suggestion_classifier_cooldown_secs) * 1_000,
        allowed_cues,
        context_blocks: settings.suggestion_context_blocks as usize,
        // Janela maior: o modelo precisa de fio de conversa para reconhecer
        // uma pergunta repetida e entender o contexto do agente.
        window_ms: 300_000,
        max_segments: 120,
        answered_questions: settings.suggestion_answered_questions as usize,
        locale: settings
            .ui_language
            .clone()
            .unwrap_or_else(|| "pt".to_string()),
        mode_id: settings.assistant_mode_id.clone(),
        mode_context: selected_mode_context(settings),
        allow_single_speaker: settings.suggestion_scope != "blocks",
        they_only: settings.suggestion_scope == "they_only",
    }
}

#[derive(Debug, Clone)]
struct Generated {
    block_id: String,
    at_ms: i64,
}

/// Estado em memória por sessão.
#[derive(Default)]
struct GateState {
    /// Última classificação (mesmo negativa).
    last: HashMap<String, ClassificationOutcome>,
    /// Último bloco que gerou sugestão (dedupe/cooldown).
    generated: HashMap<String, Generated>,
    /// Decisão positiva aguardando o cooldown.
    pending: HashMap<String, ClassificationOutcome>,
    /// Perguntas já respondidas (mais recentes primeiro), por sessão.
    answered_questions: HashMap<String, Vec<PriorQuestion>>,
    /// Título da pergunta do bloco, aguardando `mark_generated` confirmar a
    /// resposta. Sem isso, uma classificação sem geração não entra no histórico.
    question_titles: HashMap<String, String>,
}

/// Implementa o gate sobre as portas resolvidas.
pub struct SuggestionGateService {
    classifier: Arc<dyn SuggestionCueClassifier>,
    transcriptions: Arc<dyn TranscriptionsRepository>,
    /// `started_at_ms` da sessão: as falas guardam ms **relativos** ao início,
    /// enquanto o relógio do sistema é época Unix. A janela/silêncio precisam
    /// da mesma base das falas.
    sessions: Arc<dyn SessionRepository>,
    clock: Arc<dyn Clock>,
    publisher: Arc<dyn EventPublisher>,
    /// Configuração mutável: salvar as settings aplica sem reiniciar (task 06).
    config: RwLock<SuggestionGateConfig>,
    state: Mutex<GateState>,
}

impl SuggestionGateService {
    pub fn new(
        classifier: Arc<dyn SuggestionCueClassifier>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
        sessions: Arc<dyn SessionRepository>,
        clock: Arc<dyn Clock>,
        publisher: Arc<dyn EventPublisher>,
        config: SuggestionGateConfig,
    ) -> Self {
        Self {
            classifier,
            transcriptions,
            sessions,
            clock,
            publisher,
            config: RwLock::new(config),
            state: Mutex::new(GateState::default()),
        }
    }

    /// Agora no **mesmo relógio das falas** (ms desde o início da sessão).
    ///
    /// Sem sessão (testes/painel antes de iniciar), mantém o relógio direto
    /// para não quebrar dobras que passam tempos já relativos.
    fn relative_now(&self, session_id: &str) -> i64 {
        let wall = self.clock.now_ms();
        match self
            .sessions
            .get(session_id)
            .ok()
            .flatten()
            .map(|session| session.started_at_ms)
        {
            Some(started) if started > 0 => wall.saturating_sub(started).max(0),
            _ => wall,
        }
    }

    /// Cópia da configuração atual (o lock não é mantido durante I/O).
    fn config(&self) -> SuggestionGateConfig {
        self.config
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn lock_state(&self) -> MutexGuard<'_, GateState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn publish(
        &self,
        session_id: &str,
        state: SuggestionClassifierState,
        outcome: Option<ClassificationOutcome>,
        error: Option<String>,
        generating: bool,
    ) {
        self.publisher
            .emit_suggestion_classification(&SuggestionClassificationEvent {
                session_id: session_id.to_string(),
                state,
                outcome,
                error,
                generating,
            });
    }

    fn last_block(
        blocks: &[ConversationBlock],
        min_chars: usize,
        allow_single_speaker: bool,
        they_only: bool,
    ) -> Option<(usize, &ConversationBlock)> {
        if they_only {
            // Só a ponta da conversa interessa: se o ME falou por último, não
            // há pergunta de THEY pendente. Não volta para um bloco antigo.
            let (index, block) = blocks.iter().enumerate().next_back()?;
            if block.is_open || !block.is_classifiable_with(min_chars, true) {
                return None;
            }
            return Some((index, block));
        }
        blocks
            .iter()
            .enumerate()
            .rev()
            .find(|(_, block)| block.is_classifiable_with(min_chars, allow_single_speaker))
    }
}

fn attach_metadata(
    mut outcome: ClassificationOutcome,
    block: &ConversationBlock,
    now_ms: i64,
    backend_id: &str,
) -> ClassificationOutcome {
    outcome.block_id = block.id.clone();
    outcome.session_id = block.session_id.clone();
    outcome.block_start_ms = block.start_ms;
    outcome.block_end_ms = block.end_ms;
    outcome.turn_count = block.turns.len();
    outcome.classified_at_ms = now_ms;
    if outcome.backend.trim().is_empty() {
        outcome.backend = backend_id.to_string();
    }
    outcome.normalized()
}

fn remember_answered(
    state: &mut GateState,
    session_id: &str,
    title: String,
    block_id: &str,
    at_ms: i64,
    limit: usize,
) {
    if title.trim().is_empty() || limit == 0 {
        return;
    }
    let history = state
        .answered_questions
        .entry(session_id.to_string())
        .or_default();
    let normalized = title.to_lowercase();
    history.retain(|prior| prior.title.to_lowercase() != normalized);
    history.insert(
        0,
        PriorQuestion {
            title,
            block_id: block_id.to_string(),
            answered_at_ms: at_ms,
        }
        .normalized(),
    );
    history.truncate(limit);
}

impl SuggestionGate for SuggestionGateService {
    fn classify_latest(&self, session_id: &str) -> GateResult<ClassificationOutcome> {
        let config = self.config();
        let wall_now = self.clock.now_ms();
        let now = self.relative_now(session_id);
        let since = now.saturating_sub(config.window_ms.max(0));
        let segments = self
            .transcriptions
            .list_recent_by_session(session_id, since, config.max_segments)
            .map_err(|error| SuggestionGateError::Repository(error.to_string()))?;
        let blocks = build_blocks(session_id, &segments, &config.block, now);

        let Some((index, block)) = Self::last_block(
            &blocks,
            config.block.min_chars,
            config.allow_single_speaker,
            config.they_only,
        ) else {
            return Err(SuggestionGateError::NoBlock);
        };

        // O alvo é só o que THEY falou desde a última fala de ME. O bloco
        // completo continua disponível em `full_block` como contexto.
        let target =
            they_target(block, config.heuristic.min_words).ok_or(SuggestionGateError::NoBlock)?;

        if let Some(cached) = self.lock_state().last.get(session_id) {
            if cached.block_id == target.id {
                return Ok(cached.clone());
            }
        }

        let context_start = index.saturating_sub(config.context_blocks);
        let context = blocks[context_start..index].to_vec();
        let prior_questions = self
            .lock_state()
            .answered_questions
            .get(session_id)
            .cloned()
            .unwrap_or_default();
        let request = ClassificationRequest {
            session_id: session_id.to_string(),
            block: target.clone(),
            full_block: block.clone(),
            context,
            locale: config.locale.clone(),
            mode_id: config.mode_id.clone(),
            mode_context: config.mode_context.clone(),
            allow_single_speaker: config.allow_single_speaker,
            prior_questions,
            now_ms: now,
        };

        self.publish(
            session_id,
            SuggestionClassifierState::Analyzing,
            None,
            None,
            false,
        );

        match self.classifier.classify(&request) {
            Ok(outcome) => {
                let outcome =
                    attach_metadata(outcome, block, wall_now, self.classifier.backend_id());
                {
                    let mut state = self.lock_state();
                    if outcome.cue.can_suggest() {
                        state
                            .question_titles
                            .insert(outcome.block_id.clone(), question_title(&target));
                    } else {
                        state.question_titles.remove(&outcome.block_id);
                    }
                    if outcome.cue == SuggestionCue::AlreadyAnswered {
                        // A pergunta foi respondida mesmo sem gerar sugestão
                        // (ex.: ME respondeu antes do copiloto). Registra para
                        // que uma repetição futura não seja sugerida.
                        remember_answered(
                            &mut state,
                            session_id,
                            question_title(&target),
                            &outcome.block_id,
                            now,
                            config.answered_questions,
                        );
                    }
                    state.last.insert(session_id.to_string(), outcome.clone());
                }
                self.publish(
                    session_id,
                    SuggestionClassifierState::Ready,
                    Some(outcome.clone()),
                    None,
                    false,
                );
                Ok(outcome)
            }
            Err(error) => {
                tracing::warn!("classificação de sugestão falhou: {error}");
                let last = self.last(session_id);
                self.publish(
                    session_id,
                    SuggestionClassifierState::Error,
                    last,
                    Some(error.to_string()),
                    false,
                );
                Err(SuggestionGateError::Classifier(error))
            }
        }
    }

    fn last(&self, session_id: &str) -> Option<ClassificationOutcome> {
        self.lock_state().last.get(session_id).cloned()
    }

    fn pending(&self, session_id: &str) -> Option<ClassificationOutcome> {
        self.lock_state().pending.get(session_id).cloned()
    }

    fn take_pending(&self, session_id: &str) -> Option<ClassificationOutcome> {
        self.lock_state().pending.remove(session_id)
    }

    fn should_generate(
        &self,
        outcome: &ClassificationOutcome,
        now_ms: i64,
        generating: bool,
    ) -> GateDecision {
        let (last_generated_block_id, elapsed_since_last_suggestion_ms) = {
            let state = self.lock_state();
            match state.generated.get(&outcome.session_id) {
                Some(generated) => (
                    Some(generated.block_id.clone()),
                    now_ms.saturating_sub(generated.at_ms).max(0),
                ),
                None => (None, i64::MAX),
            }
        };

        let config = self.config();
        let decision = decide(&GateTick {
            enabled: config.enabled,
            has_session: true,
            cue: outcome.cue,
            should_suggest: outcome.should_suggest,
            confidence: outcome.confidence,
            cue_allowed: config.allows(outcome.cue),
            confidence_min: config.confidence_min,
            cooldown_ms: config.cooldown_ms,
            elapsed_since_last_suggestion_ms,
            last_generated_block_id,
            block_id: outcome.block_id.clone(),
            generating,
        });

        let mut state = self.lock_state();
        match decision {
            GateDecision::Generate => {
                state.pending.remove(&outcome.session_id);
            }
            GateDecision::Skip(GateSkipReason::Cooldown { .. }) => {
                state
                    .pending
                    .insert(outcome.session_id.clone(), outcome.clone());
            }
            GateDecision::Skip(GateSkipReason::Generating) => {
                // A geração em curso não invalida a decisão pendente.
            }
            GateDecision::Skip(_) => {
                state.pending.remove(&outcome.session_id);
            }
        }
        decision
    }

    fn mark_generated(&self, session_id: &str, block_id: &str, at_ms: i64) {
        let limit = self.config().answered_questions;
        let mut state = self.lock_state();
        state.generated.insert(
            session_id.to_string(),
            Generated {
                block_id: block_id.to_string(),
                at_ms,
            },
        );
        let Some(title) = state.question_titles.remove(block_id) else {
            return;
        };
        remember_answered(&mut state, session_id, title, block_id, at_ms, limit);
    }

    fn reconfigure(&self, config: SuggestionGateConfig) {
        *self
            .config
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = config;
    }

    fn reset(&self, session_id: &str) {
        let mut state = self.lock_state();
        state.last.remove(session_id);
        state.generated.remove(session_id);
        state.pending.remove(session_id);
        state.answered_questions.remove(session_id);
        state.question_titles.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::adapters::ai::fake_suggestion_cue_classifier::FakeSuggestionCueClassifier;
    use crate::adapters::ai::heuristic_suggestion_cue_classifier::HeuristicSuggestionCueClassifier;
    use crate::adapters::persistence::memory::in_memory_session_repository::InMemorySessionRepository;
    use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;
    use crate::adapters::system::system_clock::FixedClock;
    use crate::domain::audio::AudioLevel;
    use crate::domain::session::{Session, SessionStatus};
    use crate::domain::suggestion_gate::{SuggestionCue, SuggestionMode};
    use crate::domain::transcription::{AsrStatus, Speaker, TranscriptSegment};
    use crate::ports::event_publisher::{
        JournalUpdate, LlmSourceUsage, SessionStateEvent, SuggestionCadenceEvent,
    };
    use crate::ports::session_controller::AudioChannel;
    use crate::ports::suggestion_generator::Suggestion;

    #[derive(Default)]
    struct RecordingPublisher {
        events: Mutex<Vec<SuggestionClassificationEvent>>,
    }

    impl RecordingPublisher {
        fn events(&self) -> Vec<SuggestionClassificationEvent> {
            self.events
                .lock()
                .map(|events| events.clone())
                .unwrap_or_default()
        }
    }

    impl EventPublisher for RecordingPublisher {
        fn emit_segment(&self, _segment: &TranscriptSegment) {}
        fn emit_audio_level(&self, _channel: AudioChannel, _level: AudioLevel) {}
        fn emit_session_state(&self, _state: &SessionStateEvent) {}
        fn emit_insight(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion_cadence(&self, _cadence: &SuggestionCadenceEvent) {}
        fn emit_suggestion_classification(&self, event: &SuggestionClassificationEvent) {
            if let Ok(mut events) = self.events.lock() {
                events.push(event.clone());
            }
        }
        fn emit_asr_status(&self, _status: &AsrStatus) {}
        fn emit_llm_source_used(&self, _usage: &LlmSourceUsage) {}
        fn emit_journal(&self, _update: &JournalUpdate) {}
    }

    fn segment(id: &str, speaker: Speaker, text: &str, start: i64, end: i64) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            session_id: "s1".to_string(),
            speaker,
            text: text.to_string(),
            start_ms: start,
            end_ms: end,
            is_final: true,
        }
    }

    fn config() -> SuggestionGateConfig {
        SuggestionGateConfig {
            block: crate::domain::suggestion_gate::BlockConfig {
                gap_ms: 1_000,
                max_block_ms: 100_000,
                max_turns: 10,
                min_chars: 1,
                context_blocks: 8,
            },
            ..SuggestionGateConfig::default()
        }
    }

    fn outcome(cue: SuggestionCue, should_suggest: bool) -> ClassificationOutcome {
        ClassificationOutcome {
            block_id: "wrong".to_string(),
            session_id: "wrong".to_string(),
            cue,
            should_suggest,
            confidence: 0.9,
            evidence: "evidência".to_string(),
            focus: Some("foco".to_string()),
            backend: "fake".to_string(),
            model: None,
            latency_ms: 1,
            block_start_ms: 0,
            block_end_ms: 0,
            turn_count: 0,
            classified_at_ms: 0,
            abstained: false,
        }
    }

    fn service(
        classifier: Arc<FakeSuggestionCueClassifier>,
        repo: Arc<InMemoryTranscriptionsRepository>,
        publisher: Arc<RecordingPublisher>,
    ) -> SuggestionGateService {
        SuggestionGateService::new(
            classifier,
            repo,
            Arc::new(InMemorySessionRepository::new()),
            Arc::new(FixedClock(10_000)),
            publisher,
            config(),
        )
    }

    fn seed_exchange(repo: &InMemoryTranscriptionsRepository) {
        // Termina em THEY: o alvo do classificador é a pergunta/convite pendente.
        repo.insert(&segment("a", Speaker::Me, "vamos revisar o plano", 0, 500))
            .unwrap();
        repo.insert(&segment(
            "b",
            Speaker::Me,
            "sim, podemos revisar",
            1_000,
            1_500,
        ))
        .unwrap();
        repo.insert(&segment(
            "c",
            Speaker::They,
            "podemos revisar agora?",
            2_000,
            2_500,
        ))
        .unwrap();
    }

    #[test]
    fn uses_session_relative_time_for_transcriptions() {
        // Regressão do app real: `transcriptions.end_ms` é relativo ao início
        // da sessão, mas o relógio do sistema é época Unix. O gate precisa
        // converter antes de aplicar a janela/silêncio.
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        seed_exchange(&repo);
        let sessions = Arc::new(InMemorySessionRepository::new());
        sessions
            .create(&Session {
                id: "s1".to_string(),
                started_at_ms: 1_790_000_000_000,
                ended_at_ms: None,
                provider: "local".to_string(),
                model: None,
                transcription_model: None,
                user_id: None,
                subject: None,
                overview: None,
                status: SessionStatus::Active,
            })
            .expect("sessão");
        let gate = SuggestionGateService::new(
            classifier,
            repo,
            sessions,
            Arc::new(FixedClock(1_790_000_010_000)),
            Arc::new(RecordingPublisher::default()),
            config(),
        );

        let result = gate.classify_latest("s1").expect("classificação");
        assert_eq!(result.block_id, "blk_s1_0_2500_c");
        assert_eq!(result.classified_at_ms, 1_790_000_010_000);
    }

    #[test]
    fn adaptive_scope_classifies_a_single_speaker_block() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Comment, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        repo.insert(&segment(
            "a",
            Speaker::They,
            "o que você acha do banco?",
            0,
            5_000,
        ))
        .unwrap();

        let mut adaptive = config();
        adaptive.allow_single_speaker = true;
        let gate = service_at(
            classifier.clone(),
            repo.clone(),
            Arc::new(FixedClock(10_000)),
            Arc::new(RecordingPublisher::default()),
            adaptive,
        );
        let result = gate.classify_latest("s1").expect("adaptive");
        assert_eq!(result.cue, SuggestionCue::Comment);
        assert!(!classifier.requests()[0].block.has_both_speakers());
        assert!(!classifier.requests()[0].full_block.has_both_speakers());
        assert!(classifier.requests()[0]
            .block
            .turns
            .iter()
            .all(|turn| turn.speaker == Speaker::They));

        // O escopo padrão (blocos) continua exigindo os dois falantes.
        let gate_blocks = service_at(
            Arc::new(FakeSuggestionCueClassifier::new()),
            repo,
            Arc::new(FixedClock(10_000)),
            Arc::new(RecordingPublisher::default()),
            config(),
        );
        assert!(matches!(
            gate_blocks.classify_latest("s1"),
            Err(SuggestionGateError::NoBlock)
        ));
    }

    #[test]
    fn they_only_ignores_me_and_projects_only_they_turns() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        repo.insert(&segment("a", Speaker::Me, "minha resposta longa", 0, 500))
            .unwrap();

        let mut they_only = config();
        they_only.allow_single_speaker = true;
        they_only.they_only = true;

        // O bloco mais novo termina em ME: nada a classificar.
        let gate = service_at(
            classifier.clone(),
            repo.clone(),
            Arc::new(FixedClock(2_000)),
            Arc::new(RecordingPublisher::default()),
            they_only.clone(),
        );
        assert!(matches!(
            gate.classify_latest("s1"),
            Err(SuggestionGateError::NoBlock)
        ));

        // THEY pergunta: o bloco projetado tem só turnos de THEY.
        repo.insert(&segment("b", Speaker::They, "e o prazo?", 4_000, 4_500))
            .unwrap();
        let gate = service_at(
            classifier.clone(),
            repo,
            Arc::new(FixedClock(10_000)),
            Arc::new(RecordingPublisher::default()),
            they_only,
        );
        let result = gate.classify_latest("s1").expect("classificação");
        assert_eq!(result.cue, SuggestionCue::Question);
        let request = &classifier.requests()[0];
        assert!(!request.block.turns.is_empty());
        assert!(request
            .block
            .turns
            .iter()
            .all(|turn| turn.speaker == Speaker::They));
        assert_eq!(request.block.me_chars, 0);
    }

    #[test]
    fn they_only_does_not_reopen_a_question_answered_before_the_last_they_turn() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::NoAction, false));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        repo.insert(&segment("a", Speaker::They, "qual é o prazo?", 0, 500))
            .unwrap();
        repo.insert(&segment("b", Speaker::Me, "entrego na sexta", 1_000, 1_500))
            .unwrap();
        repo.insert(&segment("c", Speaker::They, "obrigado", 2_000, 2_500))
            .unwrap();
        let mut they_only = config();
        they_only.allow_single_speaker = true;
        they_only.they_only = true;
        let gate = service_at(
            classifier.clone(),
            repo,
            Arc::new(FixedClock(10_000)),
            Arc::new(RecordingPublisher::default()),
            they_only,
        );

        let _ = gate.classify_latest("s1").expect("classificação");
        let request = &classifier.requests()[0];
        assert_eq!(request.block.turns.len(), 1);
        assert_eq!(request.block.turns[0].text, "obrigado");
    }

    #[test]
    fn short_me_noise_does_not_block_the_they_question() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        repo.insert(&segment("a", Speaker::They, "o que você faz?", 0, 1_000))
            .unwrap();
        repo.insert(&segment("b", Speaker::Me, "tchau tchau", 1_100, 1_400))
            .unwrap();
        let mut cfg = config();
        cfg.allow_single_speaker = true;
        let gate = service_at(
            classifier.clone(),
            repo.clone(),
            Arc::new(FixedClock(10_000)),
            Arc::new(RecordingPublisher::default()),
            cfg.clone(),
        );

        let result = gate.classify_latest("s1").expect("ruído não bloqueia");
        assert_eq!(result.cue, SuggestionCue::Question);
        assert_eq!(classifier.requests()[0].block.turns.len(), 1);
        assert_eq!(
            classifier.requests()[0].block.turns[0].text,
            "o que você faz?"
        );

        // Resposta real do ME (3+ palavras) encerra a pergunta.
        repo.insert(&segment(
            "c",
            Speaker::Me,
            "eu entrego na polícia",
            2_000,
            2_500,
        ))
        .unwrap();
        let gate = service_at(
            classifier,
            repo,
            Arc::new(FixedClock(10_000)),
            Arc::new(RecordingPublisher::default()),
            cfg,
        );
        assert!(matches!(
            gate.classify_latest("s1"),
            Err(SuggestionGateError::NoBlock)
        ));
    }

    #[test]
    fn config_uses_the_selected_agent_scope_and_laya_threshold() {
        use crate::domain::settings::Settings;

        let settings = Settings {
            assistant_mode_id: "technical_interview".to_string(),
            suggestion_scope: "adaptive".to_string(),
            suggestion_classifier_backend: "laya".to_string(),
            suggestion_classifier_sensitivity: "balanced".to_string(),
            ..Settings::default()
        };
        let config = config_from_settings(&settings);
        assert!(config.allow_single_speaker);
        assert!(config.mode_context.contains("Entrevista Técnica"));
        assert_eq!(config.confidence_min, 0.50);

        let heuristic = Settings {
            suggestion_classifier_backend: "heuristic".to_string(),
            ..settings.clone()
        };
        assert_eq!(config_from_settings(&heuristic).confidence_min, 0.70);

        let they_only = Settings {
            suggestion_scope: "they_only".to_string(),
            ..settings
        };
        let config = config_from_settings(&they_only);
        assert!(config.they_only);
        assert!(config.allow_single_speaker);
    }

    #[test]
    fn classifies_newest_closed_block_with_both_speakers() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        seed_exchange(&repo);
        let publisher = Arc::new(RecordingPublisher::default());
        let gate = service(classifier.clone(), repo, publisher.clone());

        let result = gate.classify_latest("s1").expect("classificação");
        assert_eq!(result.cue, SuggestionCue::Question);
        assert_eq!(result.block_id, "blk_s1_0_2500_c");
        assert_eq!(result.session_id, "s1");
        assert_eq!(classifier.requests().len(), 1);
        assert!(classifier.requests()[0].full_block.has_both_speakers());
        assert!(classifier.requests()[0]
            .block
            .turns
            .iter()
            .all(|turn| turn.speaker == Speaker::They));
        assert!(classifier.requests()[0].prior_questions.is_empty());

        let events = publisher.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].state, SuggestionClassifierState::Analyzing);
        assert_eq!(events[1].state, SuggestionClassifierState::Ready);
    }

    #[test]
    fn does_not_classify_same_block_twice() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        seed_exchange(&repo);
        let gate = service(
            classifier.clone(),
            repo,
            Arc::new(RecordingPublisher::default()),
        );

        let first = gate.classify_latest("s1").expect("primeira");
        let second = gate.classify_latest("s1").expect("cache");
        assert_eq!(first, second);
        assert_eq!(classifier.requests().len(), 1);
    }

    fn service_at(
        classifier: Arc<FakeSuggestionCueClassifier>,
        repo: Arc<InMemoryTranscriptionsRepository>,
        clock: Arc<dyn crate::ports::clock::Clock>,
        publisher: Arc<RecordingPublisher>,
        config: SuggestionGateConfig,
    ) -> SuggestionGateService {
        SuggestionGateService::new(
            classifier,
            repo,
            Arc::new(InMemorySessionRepository::new()),
            clock,
            publisher,
            config,
        )
    }

    #[test]
    fn includes_previous_blocks_as_context() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        seed_exchange(&repo);
        repo.insert(&segment("d", Speaker::Me, "outra pergunta", 7_000, 7_500))
            .unwrap();
        repo.insert(&segment(
            "e",
            Speaker::They,
            "qual é o prazo?",
            8_000,
            8_500,
        ))
        .unwrap();
        let gate = service(
            classifier.clone(),
            repo,
            Arc::new(RecordingPublisher::default()),
        );
        let mut config = gate.config();
        config.context_blocks = 1;
        gate.reconfigure(config);

        let result = gate.classify_latest("s1").expect("classificação");
        assert_eq!(result.block_id, "blk_s1_7000_8500_e");
        assert_eq!(classifier.requests().len(), 1);
        assert_eq!(classifier.requests()[0].context.len(), 1);
    }

    #[test]
    fn last_classification_survives_no_suggestion() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::NoAction, false));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        seed_exchange(&repo);
        let gate = service(classifier, repo, Arc::new(RecordingPublisher::default()));

        let result = gate.classify_latest("s1").expect("classificação");
        assert!(!result.should_suggest);
        assert_eq!(
            gate.last("s1").map(|last| last.cue),
            Some(SuggestionCue::NoAction)
        );
    }

    #[test]
    fn classifier_error_keeps_last_valid_outcome() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        classifier.push_error(crate::ports::suggestion_cue_classifier::ClassifierError::Timeout);
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        seed_exchange(&repo);
        let publisher = Arc::new(RecordingPublisher::default());
        let gate = service(classifier, repo.clone(), publisher.clone());

        let first = gate.classify_latest("s1").expect("primeira");
        // Fala nova muda o id do bloco e força uma nova inferência (o erro).
        repo.insert(&segment("f", Speaker::Me, "podemos seguir", 7_000, 7_500))
            .unwrap();
        repo.insert(&segment(
            "g",
            Speaker::They,
            "e o próximo passo?",
            8_000,
            8_500,
        ))
        .unwrap();
        assert!(matches!(
            gate.classify_latest("s1"),
            Err(SuggestionGateError::Classifier(_))
        ));
        assert_eq!(gate.last("s1"), Some(first));
        let events = publisher.events();
        assert_eq!(
            events.last().expect("erro").state,
            SuggestionClassifierState::Error
        );
        assert!(events.last().expect("erro").outcome.is_some());
    }

    #[test]
    fn skips_when_cue_disabled_or_below_confidence() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        let gate = service(classifier, repo, Arc::new(RecordingPublisher::default()));

        let mut blocked = outcome(SuggestionCue::Question, true);
        blocked.session_id = "s1".to_string();
        blocked.block_id = "blk_1".to_string();

        let low = ClassificationOutcome {
            confidence: 0.2,
            ..blocked.clone()
        };
        assert_eq!(
            gate.should_generate(&low, 10_000, false),
            GateDecision::Skip(crate::domain::suggestion_gate::GateSkipReason::BelowConfidence)
        );

        let mut config = gate.config();
        config.allowed_cues = vec![SuggestionCue::Comment];
        gate.reconfigure(config);
        assert_eq!(
            gate.should_generate(&blocked, 10_000, false),
            GateDecision::Skip(crate::domain::suggestion_gate::GateSkipReason::CueNotAllowed)
        );
    }

    #[test]
    fn skips_cooldown_and_duplicate_block() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        let gate = service(classifier, repo, Arc::new(RecordingPublisher::default()));
        let mut decision_outcome = outcome(SuggestionCue::Question, true);
        decision_outcome.session_id = "s1".to_string();
        decision_outcome.block_id = "blk_1".to_string();

        assert_eq!(
            gate.should_generate(&decision_outcome, 10_000, false),
            GateDecision::Generate
        );

        gate.mark_generated("s1", "blk_1", 10_000);
        assert_eq!(
            gate.should_generate(&decision_outcome, 10_000, false),
            GateDecision::Skip(crate::domain::suggestion_gate::GateSkipReason::AlreadyGenerated)
        );

        let mut next = decision_outcome.clone();
        next.block_id = "blk_2".to_string();
        assert_eq!(
            gate.should_generate(&next, 12_000, false),
            GateDecision::Skip(crate::domain::suggestion_gate::GateSkipReason::Cooldown {
                remaining_ms: 13_000
            })
        );
        assert_eq!(
            gate.pending("s1").map(|pending| pending.block_id),
            Some("blk_2".to_string())
        );
    }

    #[test]
    fn pending_generates_after_cooldown_without_reclassifying() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        let gate = service(
            classifier.clone(),
            repo,
            Arc::new(RecordingPublisher::default()),
        );
        let mut decision_outcome = outcome(SuggestionCue::Question, true);
        decision_outcome.session_id = "s1".to_string();
        decision_outcome.block_id = "blk_1".to_string();

        gate.mark_generated("s1", "blk_0", 10_000);
        assert!(matches!(
            gate.should_generate(&decision_outcome, 12_000, false),
            GateDecision::Skip(crate::domain::suggestion_gate::GateSkipReason::Cooldown { .. })
        ));
        assert_eq!(
            gate.pending("s1").map(|pending| pending.block_id),
            Some("blk_1".to_string())
        );

        assert_eq!(
            gate.should_generate(&decision_outcome, 26_000, false),
            GateDecision::Generate
        );
        assert!(gate.pending("s1").is_none());
        assert!(classifier.requests().is_empty());
    }

    #[test]
    fn new_block_replaces_pending_decision() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        let gate = service(classifier, repo, Arc::new(RecordingPublisher::default()));

        let mut first = outcome(SuggestionCue::Question, true);
        first.session_id = "s1".to_string();
        first.block_id = "blk_1".to_string();
        let mut second = first.clone();
        second.block_id = "blk_2".to_string();

        gate.mark_generated("s1", "blk_0", 10_000);
        assert!(matches!(
            gate.should_generate(&first, 12_000, false),
            GateDecision::Skip(crate::domain::suggestion_gate::GateSkipReason::Cooldown { .. })
        ));
        assert!(matches!(
            gate.should_generate(&second, 12_000, false),
            GateDecision::Skip(crate::domain::suggestion_gate::GateSkipReason::Cooldown { .. })
        ));
        assert_eq!(
            gate.pending("s1").map(|pending| pending.block_id),
            Some("blk_2".to_string())
        );
    }

    #[test]
    fn generating_blocks_and_reset_clears_state() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        let gate = service(classifier, repo, Arc::new(RecordingPublisher::default()));
        let mut decision_outcome = outcome(SuggestionCue::Question, true);
        decision_outcome.session_id = "s1".to_string();
        decision_outcome.block_id = "blk_1".to_string();

        assert_eq!(
            gate.should_generate(&decision_outcome, 10_000, true),
            GateDecision::Skip(crate::domain::suggestion_gate::GateSkipReason::Generating)
        );

        gate.mark_generated("s1", "blk_1", 10_000);
        gate.reset("s1");
        assert_eq!(
            gate.should_generate(&decision_outcome, 10_000, false),
            GateDecision::Generate
        );
        assert!(gate.last("s1").is_none());
    }

    #[test]
    fn generated_question_enters_the_answered_history() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        repo.insert(&segment(
            "a",
            Speaker::Me,
            "o desconto padrão é 10%",
            0,
            500,
        ))
        .unwrap();
        repo.insert(&segment(
            "b",
            Speaker::They,
            "e o desconto para 50 licenças?",
            1_000,
            1_500,
        ))
        .unwrap();
        let gate = service_at(
            classifier.clone(),
            repo.clone(),
            Arc::new(FixedClock(10_000)),
            Arc::new(RecordingPublisher::default()),
            config(),
        );

        let first = gate.classify_latest("s1").expect("primeira");
        gate.mark_generated("s1", &first.block_id, 10_000);
        // Só depois de responder a primeira pergunta entra um bloco novo.
        repo.insert(&segment("c", Speaker::Me, "podemos seguir", 3_000, 3_500))
            .unwrap();
        repo.insert(&segment(
            "d",
            Speaker::They,
            "e o prazo da segunda fase?",
            4_000,
            4_500,
        ))
        .unwrap();
        let _second = gate.classify_latest("s1").expect("segunda");

        let requests = classifier.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].prior_questions.len(), 1);
        assert!(requests[1].prior_questions[0]
            .title
            .contains("desconto para 50"));
        assert_eq!(requests[1].prior_questions[0].block_id, first.block_id);
    }

    #[test]
    fn a_generated_comment_also_enters_the_answered_history() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Comment, true));
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        repo.insert(&segment("a", Speaker::Me, "o mutex é um bloqueio", 0, 500))
            .unwrap();
        repo.insert(&segment(
            "b",
            Speaker::They,
            "o mutex é a mesma coisa que uma fila",
            1_000,
            2_000,
        ))
        .unwrap();
        repo.insert(&segment(
            "c",
            Speaker::They,
            "quais são as diferenças?",
            2_500,
            3_000,
        ))
        .unwrap();
        let gate = service_at(
            classifier.clone(),
            repo.clone(),
            Arc::new(FixedClock(20_000)),
            Arc::new(RecordingPublisher::default()),
            config(),
        );

        let first = gate.classify_latest("s1").expect("primeira");
        assert_eq!(first.cue, SuggestionCue::Comment);
        gate.mark_generated("s1", &first.block_id, 10_000);

        repo.insert(&segment(
            "d",
            Speaker::They,
            "o mutex é a mesma coisa que o mutex",
            10_000,
            11_000,
        ))
        .unwrap();
        repo.insert(&segment("e", Speaker::Me, "não é uma fila", 11_500, 12_000))
            .unwrap();
        repo.insert(&segment(
            "f",
            Speaker::They,
            "uma fila. quais são as diferenças?",
            12_500,
            13_000,
        ))
        .unwrap();
        let _second = gate.classify_latest("s1").expect("segunda");

        let requests = classifier.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].prior_questions.len(), 1);
        assert!(requests[1].prior_questions[0].title.contains("diferenças"));
    }

    #[test]
    fn an_already_answered_block_enters_history_without_generation() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::AlreadyAnswered, false));
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        repo.insert(&segment("a", Speaker::Me, "vamos revisar", 0, 500))
            .unwrap();
        repo.insert(&segment(
            "b",
            Speaker::They,
            "o que é um mutex?",
            1_000,
            2_000,
        ))
        .unwrap();
        let mut cfg = config();
        cfg.allow_single_speaker = true;
        let gate = service_at(
            classifier.clone(),
            repo.clone(),
            Arc::new(FixedClock(20_000)),
            Arc::new(RecordingPublisher::default()),
            cfg,
        );

        let first = gate.classify_latest("s1").expect("primeira");
        assert_eq!(first.cue, SuggestionCue::AlreadyAnswered);
        // Sem `mark_generated`: a resposta do ME já conta como respondida.
        assert!(gate.last("s1").is_some());

        repo.insert(&segment("c", Speaker::Me, "podemos seguir", 5_000, 5_500))
            .unwrap();
        repo.insert(&segment(
            "d",
            Speaker::They,
            "e o que é uma fila?",
            6_000,
            6_500,
        ))
        .unwrap();
        let _second = gate.classify_latest("s1").expect("segunda");
        let requests = classifier.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].prior_questions.len(), 1);
        assert!(requests[1].prior_questions[0].title.contains("mutex"));
    }

    #[test]
    fn settings_map_to_the_gate_config() {
        let defaults = crate::domain::settings::Settings::default();
        let config = config_from_settings(&defaults);
        assert!(config.enabled);
        assert_eq!(config.confidence_min, 0.70);
        assert_eq!(config.cooldown_ms, 15_000);
        assert!(config.allowed_cues.is_empty());
        assert_eq!(config.block.gap_ms, 3_000);
        assert_eq!(config.block.max_turns, 12);
        assert_eq!(config.context_blocks, 8);

        let high = crate::domain::settings::Settings {
            suggestion_classifier_sensitivity: "high".into(),
            suggestion_allowed_cues: vec!["question".into(), "invitation".into()],
            ..crate::domain::settings::Settings::default()
        };
        let config = config_from_settings(&high);
        assert_eq!(config.confidence_min, 0.55);
        assert_eq!(
            config.allowed_cues,
            vec![SuggestionCue::Question, SuggestionCue::Comment]
        );
    }

    #[test]
    fn demo_prints_heuristic_classifications() {
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        let add = |session: &str, speaker: Speaker, text: &str, start: i64, end: i64| {
            repo.insert(&TranscriptSegment {
                id: format!("{session}-{start}"),
                session_id: session.to_string(),
                speaker,
                text: text.to_string(),
                start_ms: start,
                end_ms: end,
                is_final: true,
            })
            .expect("segmento");
        };
        add("s1", Speaker::Me, "eu consigo explicar", 0, 500);
        add("s1", Speaker::They, "pode explicar melhor?", 1_000, 1_500);
        add("s2", Speaker::Me, "é, tipo assim, eu acho", 0, 2_000);
        add("s2", Speaker::They, "certo", 2_500, 2_800);
        add("s3", Speaker::Me, "vamos revisar o cronograma", 0, 1_000);
        add("s3", Speaker::They, "concordo com o plano", 1_500, 2_000);
        add(
            "s3",
            Speaker::Me,
            "fechamos os proximos passos",
            2_500,
            3_000,
        );
        add("s3", Speaker::They, "e o próximo passo?", 3_500, 4_000);
        add("s4", Speaker::Me, "vamos revisar o cronograma", 0, 1_000);
        add("s4", Speaker::They, "concordo com o plano", 1_500, 2_000);

        let gate = SuggestionGateService::new(
            Arc::new(HeuristicSuggestionCueClassifier::default()),
            repo,
            Arc::new(InMemorySessionRepository::new()),
            Arc::new(FixedClock(10_000)),
            Arc::new(RecordingPublisher::default()),
            config(),
        );

        for session in ["s1", "s2", "s3", "s4"] {
            let outcome = gate.classify_latest(session).expect("classificação");
            let decision = gate.should_generate(&outcome, 10_000, false);
            println!(
                "[{session}] cue={} should_suggest={} confidence={:.2} decision={decision:?}",
                outcome.cue.as_str(),
                outcome.should_suggest,
                outcome.confidence
            );
        }
    }

    #[test]
    fn mode_type_is_available_to_the_application() {
        let mode = SuggestionMode::Classifier;
        assert_eq!(mode.as_str(), "classifier");
    }
}
