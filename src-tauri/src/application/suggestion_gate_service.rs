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
    build_blocks, decide, open_probe_eligible, BlockConfig, ClassifierSensitivity,
    ConversationBlock, GateDecision, GateSkipReason, GateTick, HeuristicConfig, OpenProbeConfig,
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
        window_ms: 180_000,
        max_segments: 40,
        probe_open_block: settings.suggestion_probe_open_block,
        hesitation_probe_ms: i64::from(settings.suggestion_hesitation_probe_ms),
        probe_min_words: 3,
        probe_throttle_ms: 2_000,
        locale: settings
            .ui_language
            .clone()
            .unwrap_or_else(|| "pt".to_string()),
        mode_id: settings.assistant_mode_id.clone(),
        mode_context: selected_mode_context(settings),
        allow_single_speaker: settings.suggestion_scope == "adaptive",
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
    /// Última sonda de bloco aberto por sessão (throttle).
    last_probe: HashMap<String, i64>,
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
        force_open: bool,
        min_chars: usize,
        allow_single_speaker: bool,
    ) -> Option<(usize, &ConversationBlock)> {
        if force_open {
            return blocks
                .len()
                .checked_sub(1)
                .and_then(|index| blocks.get(index).map(|block| (index, block)));
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

impl SuggestionGate for SuggestionGateService {
    fn probe_open(&self, session_id: &str) -> GateResult<Option<ClassificationOutcome>> {
        let config = self.config();
        let now = self.relative_now(session_id);
        let since = now.saturating_sub(config.window_ms.max(0));
        let segments = self
            .transcriptions
            .list_recent_by_session(session_id, since, config.max_segments)
            .map_err(|error| SuggestionGateError::Repository(error.to_string()))?;
        let blocks = build_blocks(session_id, &segments, &config.block, now);
        let Some(block) = blocks.last() else {
            return Ok(None);
        };

        let last_probe = {
            let state = self.lock_state();
            state.last_probe.get(session_id).copied()
        };
        let probe_config = OpenProbeConfig {
            enabled: config.probe_open_block,
            hesitation_allowed: config.allows(SuggestionCue::Hesitation),
            min_words: config.probe_min_words,
            hesitation_ms: config.hesitation_probe_ms,
            throttle_ms: config.probe_throttle_ms,
        };
        if !open_probe_eligible(block, now, last_probe, &probe_config) {
            return Ok(None);
        }

        // Sem fala nova desde a última classificação deste bloco: não paga
        // outra inferência (o id muda quando entra um segmento novo).
        if self
            .lock_state()
            .last
            .get(session_id)
            .is_some_and(|last| last.block_id == block.id)
        {
            return Ok(None);
        }

        self.lock_state()
            .last_probe
            .insert(session_id.to_string(), now);
        self.classify_latest(session_id, true).map(Some)
    }

    fn classify_latest(
        &self,
        session_id: &str,
        force_open: bool,
    ) -> GateResult<ClassificationOutcome> {
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
            force_open,
            config.block.min_chars,
            config.allow_single_speaker,
        ) else {
            return Err(SuggestionGateError::NoBlock);
        };

        if !force_open {
            if let Some(cached) = self.lock_state().last.get(session_id) {
                if cached.block_id == block.id {
                    return Ok(cached.clone());
                }
            }
        }

        let context_start = index.saturating_sub(config.context_blocks);
        let context = blocks[context_start..index].to_vec();
        let request = ClassificationRequest {
            session_id: session_id.to_string(),
            block: block.clone(),
            context,
            locale: config.locale.clone(),
            mode_id: config.mode_id.clone(),
            mode_context: config.mode_context.clone(),
            allow_single_speaker: config.allow_single_speaker,
            is_open_probe: force_open,
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
                self.lock_state()
                    .last
                    .insert(session_id.to_string(), outcome.clone());
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
        self.lock_state().generated.insert(
            session_id.to_string(),
            Generated {
                block_id: block_id.to_string(),
                at_ms,
            },
        );
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
        state.last_probe.remove(session_id);
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
    use crate::adapters::system::system_clock::{FixedClock, MutableClock};
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
                context_blocks: 3,
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
        repo.insert(&segment("a", Speaker::Me, "vamos revisar o plano", 0, 500))
            .unwrap();
        repo.insert(&segment(
            "b",
            Speaker::They,
            "podemos revisar agora?",
            1_000,
            1_500,
        ))
        .unwrap();
        repo.insert(&segment(
            "c",
            Speaker::Me,
            "sim, podemos revisar",
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

        let result = gate.classify_latest("s1", false).expect("classificação");
        assert_eq!(result.block_id, "blk_s1_0_2500_c");
        assert_eq!(result.classified_at_ms, 1_790_000_010_000);
    }

    #[test]
    fn adaptive_scope_classifies_a_single_speaker_block() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Hesitation, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        repo.insert(&segment(
            "a",
            Speaker::Me,
            "Eu não tenho certeza, deixa eu pensar",
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
        let result = gate.classify_latest("s1", false).expect("adaptive");
        assert_eq!(result.cue, SuggestionCue::Hesitation);
        assert!(!classifier.requests()[0].block.has_both_speakers());

        // O escopo padrão (blocos) continua exigindo os dois falantes.
        let gate_blocks = service_at(
            Arc::new(FakeSuggestionCueClassifier::new()),
            repo,
            Arc::new(FixedClock(10_000)),
            Arc::new(RecordingPublisher::default()),
            config(),
        );
        assert!(matches!(
            gate_blocks.classify_latest("s1", false),
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
            ..settings
        };
        assert_eq!(config_from_settings(&heuristic).confidence_min, 0.70);
    }

    #[test]
    fn classifies_newest_closed_block_with_both_speakers() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Question, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        seed_exchange(&repo);
        let publisher = Arc::new(RecordingPublisher::default());
        let gate = service(classifier.clone(), repo, publisher.clone());

        let result = gate.classify_latest("s1", false).expect("classificação");
        assert_eq!(result.cue, SuggestionCue::Question);
        assert_eq!(result.block_id, "blk_s1_0_2500_c");
        assert_eq!(result.session_id, "s1");
        assert_eq!(classifier.requests().len(), 1);
        assert!(classifier.requests()[0].block.has_both_speakers());
        assert!(!classifier.requests()[0].is_open_probe);

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

        let first = gate.classify_latest("s1", false).expect("primeira");
        let second = gate.classify_latest("s1", false).expect("cache");
        assert_eq!(first, second);
        assert_eq!(classifier.requests().len(), 1);
    }

    fn probe_config() -> SuggestionGateConfig {
        SuggestionGateConfig {
            block: crate::domain::suggestion_gate::BlockConfig {
                gap_ms: 3_000,
                ..config().block
            },
            probe_open_block: true,
            hesitation_probe_ms: 4_000,
            probe_min_words: 3,
            probe_throttle_ms: 2_000,
            ..config()
        }
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

    fn seed_open_hesitation(repo: &InMemoryTranscriptionsRepository) {
        repo.insert(&segment("a", Speaker::They, "certo", 0, 500))
            .unwrap();
        repo.insert(&segment(
            "b",
            Speaker::Me,
            "é... tipo assim... eu acho",
            1_000,
            6_000,
        ))
        .unwrap();
    }

    #[test]
    fn open_probe_triggers_hesitation_and_is_throttled() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Hesitation, true));
        classifier.push_outcome(outcome(SuggestionCue::Hesitation, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        seed_open_hesitation(&repo);
        let clock = Arc::new(MutableClock::new(6_500));
        let gate = service_at(
            classifier.clone(),
            repo.clone(),
            clock.clone(),
            Arc::new(RecordingPublisher::default()),
            probe_config(),
        );

        let first = gate.probe_open("s1").expect("sonda").expect("outcome");
        assert_eq!(first.cue, SuggestionCue::Hesitation);
        assert_eq!(classifier.requests().len(), 1);
        assert!(classifier.requests()[0].is_open_probe);
        assert!(classifier.requests()[0].block.is_open);

        // Mesmo bloco, sem fala nova: não classifica de novo.
        assert!(gate.probe_open("s1").expect("sonda").is_none());
        assert_eq!(classifier.requests().len(), 1);

        // Fala nova muda o id, mas o throttle de 2 s segura.
        repo.insert(&segment(
            "c",
            Speaker::Me,
            "e aí... como eu digo",
            6_200,
            6_400,
        ))
        .unwrap();
        assert!(gate.probe_open("s1").expect("sonda").is_none());
        assert_eq!(classifier.requests().len(), 1);

        clock.advance(2_100);
        let second = gate.probe_open("s1").expect("sonda").expect("outcome");
        assert_eq!(second.cue, SuggestionCue::Hesitation);
        assert_eq!(classifier.requests().len(), 2);
    }

    #[test]
    fn open_probe_respects_settings_and_closed_blocks() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        seed_open_hesitation(&repo);

        let mut disabled = probe_config();
        disabled.probe_open_block = false;
        let gate = service_at(
            classifier.clone(),
            repo.clone(),
            Arc::new(MutableClock::new(6_500)),
            Arc::new(RecordingPublisher::default()),
            disabled,
        );
        assert!(gate.probe_open("s1").expect("sonda").is_none());

        let mut no_hesitation = probe_config();
        no_hesitation.allowed_cues = vec![SuggestionCue::Question];
        let gate = service_at(
            classifier.clone(),
            repo.clone(),
            Arc::new(MutableClock::new(6_500)),
            Arc::new(RecordingPublisher::default()),
            no_hesitation,
        );
        assert!(gate.probe_open("s1").expect("sonda").is_none());
        assert!(classifier.requests().is_empty());

        // O bloco já fechou pelo gap: a sonda não roda.
        let gate = service_at(
            classifier.clone(),
            repo,
            Arc::new(MutableClock::new(10_000)),
            Arc::new(RecordingPublisher::default()),
            probe_config(),
        );
        assert!(gate.probe_open("s1").expect("sonda").is_none());
        assert!(classifier.requests().is_empty());
    }

    #[test]
    fn force_open_probes_the_open_block() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::Hesitation, true));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        repo.insert(&segment(
            "a",
            Speaker::Me,
            "eu estava pensando que...",
            0,
            9_500,
        ))
        .unwrap();
        let gate = service(
            classifier.clone(),
            repo,
            Arc::new(RecordingPublisher::default()),
        );

        let result = gate.classify_latest("s1", true).expect("sonda");
        assert_eq!(result.cue, SuggestionCue::Hesitation);
        assert_eq!(classifier.requests().len(), 1);
        assert!(classifier.requests()[0].is_open_probe);
        assert!(classifier.requests()[0].block.is_open);
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

        let result = gate.classify_latest("s1", false).expect("classificação");
        assert_eq!(result.block_id, "blk_s1_7000_8500_e");
        assert_eq!(classifier.requests().len(), 1);
        assert_eq!(classifier.requests()[0].context.len(), 1);
    }

    #[test]
    fn last_classification_survives_no_suggestion() {
        let classifier = Arc::new(FakeSuggestionCueClassifier::new());
        classifier.push_outcome(outcome(SuggestionCue::NoSuggestion, false));
        let repo = Arc::new(InMemoryTranscriptionsRepository::new());
        seed_exchange(&repo);
        let gate = service(classifier, repo, Arc::new(RecordingPublisher::default()));

        let result = gate.classify_latest("s1", false).expect("classificação");
        assert!(!result.should_suggest);
        assert_eq!(
            gate.last("s1").map(|last| last.cue),
            Some(SuggestionCue::NoSuggestion)
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
        let gate = service(classifier, repo, publisher.clone());

        let first = gate.classify_latest("s1", false).expect("primeira");
        assert!(matches!(
            gate.classify_latest("s1", true),
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
        config.allowed_cues = vec![SuggestionCue::Hesitation];
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
    fn settings_map_to_the_gate_config() {
        let defaults = crate::domain::settings::Settings::default();
        let config = config_from_settings(&defaults);
        assert!(config.enabled);
        assert_eq!(config.confidence_min, 0.70);
        assert_eq!(config.cooldown_ms, 15_000);
        assert!(config.allowed_cues.is_empty());
        assert_eq!(config.block.gap_ms, 3_000);
        assert_eq!(config.block.max_turns, 12);
        assert_eq!(config.context_blocks, 3);

        let high = crate::domain::settings::Settings {
            suggestion_classifier_sensitivity: "high".into(),
            suggestion_allowed_cues: vec!["question".into(), "invitation".into()],
            ..crate::domain::settings::Settings::default()
        };
        let config = config_from_settings(&high);
        assert_eq!(config.confidence_min, 0.55);
        assert_eq!(
            config.allowed_cues,
            vec![SuggestionCue::Question, SuggestionCue::Invitation]
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
            let outcome = gate.classify_latest(session, false).expect("classificação");
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
