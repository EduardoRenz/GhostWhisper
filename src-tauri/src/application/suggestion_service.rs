//! Caso de uso: sugestão contextual por agente.
//!
//! A sugestão pode ser **efêmera** (janela de sugestões) ou **persistida**
//! (botão “Sugerir”, atalho global e “Fixar”). O contexto são os últimos
//! momentos da conversa; um `since_ms` evita chamar o LLM sem fala nova.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::domain::llm::CompletionRequest;
use crate::domain::message::{ConversationMessage, MessageRole};
use crate::error::{GhostWhisperError, Result};
use crate::ports::clock::Clock;
use crate::ports::event_publisher::EventPublisher;
use crate::ports::llm_controller::LlmController;
use crate::ports::repositories::{MessagesRepository, SessionRepository, TranscriptionsRepository};
use crate::ports::settings_store::SettingsStore;
use crate::ports::suggestion_generator::{
    PinSuggestionRequest, Suggestion, SuggestionGenerator, SuggestionPhase, SuggestionRequest,
};

use super::context;
use super::prompts::{self, assistant as assistant_prompts};
use super::session_lookup::resolve_session;

/// Contexto dos “últimos momentos”.
#[derive(Debug, Clone, Copy)]
pub struct SuggestionLimits {
    pub window_ms: i64,
    pub max_segments: usize,
    pub max_context_chars: usize,
}

impl Default for SuggestionLimits {
    fn default() -> Self {
        Self {
            window_ms: 180_000,
            max_segments: 40,
            max_context_chars: 6_000,
        }
    }
}

const LAST_SUGGESTION_MAX_CHARS: usize = 2_000;
const EXTRA_CONTEXT_MAX_CHARS: usize = 4_000;
const MEETING_MAX_TOKENS: u32 = 900;
const TECHNICAL_MAX_TOKENS: u32 = 1_400;
const MEETING_TEMPERATURE: f32 = 0.4;
const TECHNICAL_TEMPERATURE: f32 = 0.2;
/// Teto do rascunho *fast-first* (Perssua: `maxOutputTokens: 240–420`).
const FAST_FIRST_MAX_TOKENS: u32 = 420;
/// Temperatura do rascunho *fast-first* (determinístico, como o Perssua).
const FAST_FIRST_TEMPERATURE: f32 = 0.0;

/// Última sugestão gerada de uma sessão (o "cursor" do contexto).
///
/// Serve para duas coisas (BR-039):
/// - **não** gerar de novo sem fala nova (nenhuma chamada ao LLM);
/// - a janela que abre no meio da conversa mostra a sugestão existente.
#[derive(Debug, Clone)]
struct LastSuggestion {
    /// Fim do contexto considerado (cursor da próxima janela).
    cursor_ms: i64,
    /// A sugestão em si (com a fase e o snapshot do agente).
    suggestion: Suggestion,
}

/// Gera sugestões contextuais por agente.
pub struct SuggestionService {
    llm: Arc<dyn LlmController>,
    settings: Arc<dyn SettingsStore>,
    sessions: Arc<dyn SessionRepository>,
    transcriptions: Arc<dyn TranscriptionsRepository>,
    messages: Arc<dyn MessagesRepository>,
    clock: Arc<dyn Clock>,
    publisher: Arc<dyn EventPublisher>,
    limits: SuggestionLimits,
    /// Última sugestão por sessão (em memória).
    last: Arc<Mutex<HashMap<String, LastSuggestion>>>,
    /// Serializa as gerações.
    ///
    /// A janela e o agendador podem pedir ao mesmo tempo; sem isso, os dois
    /// passariam pela checagem do cursor antes de qualquer um gravar a última
    /// sugestão e **duas** seriam geradas para a mesma fala.
    gate: Arc<Mutex<()>>,
}

impl SuggestionService {
    /// Cria o serviço com as portas resolvidas.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        llm: Arc<dyn LlmController>,
        settings: Arc<dyn SettingsStore>,
        sessions: Arc<dyn SessionRepository>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
        messages: Arc<dyn MessagesRepository>,
        clock: Arc<dyn Clock>,
        publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            llm,
            settings,
            sessions,
            transcriptions,
            messages,
            clock,
            publisher,
            limits: SuggestionLimits::default(),
            last: Arc::new(Mutex::new(HashMap::new())),
            gate: Arc::new(Mutex::new(())),
        }
    }

    /// Substitui os limites (testes de janela de contexto).
    #[cfg(test)]
    pub fn with_limits(mut self, limits: SuggestionLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Resolve o agente pedido (ou o selecionado nas configurações).
    fn resolve_mode(
        &self,
        requested: Option<&str>,
        settings: &crate::domain::settings::Settings,
    ) -> Result<crate::domain::assistant::AssistantMode> {
        let id = requested
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .unwrap_or(settings.assistant_mode_id.as_str());
        assistant_prompts::resolve_mode(id, &settings.custom_assistant_modes)
            .ok_or_else(|| GhostWhisperError::Config(format!("agente não encontrado: {id}")))
    }

    /// Trunca um texto opcional para o limite.
    fn clamp(value: Option<String>, max_chars: usize) -> Option<String> {
        let value = value?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed.chars().count() <= max_chars {
            return Some(trimmed.to_string());
        }
        Some(trimmed.chars().take(max_chars).collect())
    }

    /// Guarda a última sugestão da sessão (cursor + texto).
    fn remember(&self, session_id: &str, suggestion: &Suggestion) {
        if let Ok(mut last) = self.last.lock() {
            last.insert(
                session_id.to_string(),
                LastSuggestion {
                    cursor_ms: suggestion.context_until_ms,
                    suggestion: suggestion.clone(),
                },
            );
        }
    }

    /// Persiste a sugestão e devolve o id da mensagem.
    fn persist(&self, session_id: &str, at_ms: i64, text: &str) -> Result<String> {
        let message_id = Uuid::new_v4().to_string();
        self.messages.insert(&ConversationMessage {
            id: message_id.clone(),
            session_id: session_id.to_string(),
            role: MessageRole::Assistant,
            content: text.to_string(),
            timestamp_ms: at_ms,
        })?;
        Ok(message_id)
    }
}

/// Ajustes de amostragem por agente (técnicos são mais precisos e longos).
fn sampling(mode: &crate::domain::assistant::AssistantMode) -> (u32, f32) {
    match mode.id.as_str() {
        "technical_interview" | "leetcode" | "system_design" => {
            (TECHNICAL_MAX_TOKENS, TECHNICAL_TEMPERATURE)
        }
        _ => (MEETING_MAX_TOKENS, MEETING_TEMPERATURE),
    }
}

impl SuggestionGenerator for SuggestionService {
    fn generate(&self, request: SuggestionRequest) -> Result<Suggestion> {
        // Uma geração por vez (janela × agendador): a segunda encontra a
        // última sugestão gravada pela primeira e a reaproveita.
        let _gate = self
            .gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let settings = self.settings.load()?;
        let session = resolve_session(self.sessions.as_ref(), request.session_id)?;
        let mode = self.resolve_mode(request.mode_id.as_deref(), &settings)?;
        let segments = self.transcriptions.list_by_session(&session.id)?;
        let now_rel = (self.clock.now_ms() - session.started_at_ms).max(0);

        // Contexto **ao vivo**: inclui a fala em andamento, para a sugestão não
        // esperar o turno fechar (BR-028).
        let recent = context::recent_context_live(
            &segments,
            now_rel,
            self.limits.window_ms,
            self.limits.max_segments,
            self.limits.max_context_chars,
        )
        .ok_or_else(|| {
            GhostWhisperError::Config("transcrição vazia: nada para sugerir".to_string())
        })?;

        // O cursor da sessão é o **mais avançado** entre o pedido e a última
        // sugestão gerada: o agendador pode estar com um cursor velho (a janela
        // gerou depois dele) e, sem isso, a mesma fala gerava duas sugestões
        // (a duplicata do histórico — BR-039).
        let cached = self
            .last
            .lock()
            .ok()
            .and_then(|last| last.get(&session.id).cloned());
        let since = match (
            request.since_ms,
            cached.as_ref().map(|entry| entry.cursor_ms),
        ) {
            (Some(request), Some(cached)) => Some(request.max(cached)),
            (Some(request), None) => Some(request),
            (None, Some(cached)) => Some(cached),
            (None, None) => None,
        };
        if !request.force {
            if let Some(since) = since {
                if recent.until_ms <= since {
                    // Reaproveita a última sugestão: nenhuma chamada ao LLM.
                    if let Some(entry) = cached {
                        return Ok(entry.suggestion);
                    }
                    return Ok(Suggestion {
                        session_id: session.id.clone(),
                        mode_id: mode.id,
                        mode_name: mode.name,
                        mode_icon: mode.icon,
                        mode_color: mode.color,
                        phase: SuggestionPhase::Full,
                        title: String::new(),
                        text: String::new(),
                        at_ms: since,
                        context_until_ms: recent.until_ms,
                        message_id: None,
                        skipped: true,
                    });
                }
            }
        }

        let last = Self::clamp(request.last_suggestion, LAST_SUGGESTION_MAX_CHARS);
        let extra = Self::clamp(request.extra_context, EXTRA_CONTEXT_MAX_CHARS);
        let language = Some(settings.source_language.as_str());
        let user = assistant_prompts::suggestion_user_prompt(
            &recent.text,
            last.as_deref(),
            extra.as_deref(),
        );
        let (max_tokens, temperature) = sampling(&mode);

        // 1) Fast-first: rascunho curto publicado antes do texto completo.
        //    Falha aqui não derruba a sugestão (o passe completo segue).
        if request.fast_first {
            let draft_system = assistant_prompts::fast_first_system_prompt(&mode, language);
            match self.llm.complete(
                &CompletionRequest::user(user.clone())
                    .with_system(draft_system)
                    .with_temperature(FAST_FIRST_TEMPERATURE)
                    .with_max_tokens(FAST_FIRST_MAX_TOKENS),
            ) {
                Ok(completion) => {
                    // O rápido não pede `Subject:`, mas se vier, ele é o título.
                    let (title, text) = prompts::parse_suggestion_response(&completion.text);
                    if !text.is_empty() {
                        self.publisher.emit_suggestion(&Suggestion {
                            session_id: session.id.clone(),
                            mode_id: mode.id.clone(),
                            mode_name: mode.name.clone(),
                            mode_icon: mode.icon.clone(),
                            mode_color: mode.color.clone(),
                            phase: SuggestionPhase::Quick,
                            title: title.unwrap_or_default(),
                            text,
                            at_ms: now_rel,
                            context_until_ms: recent.until_ms,
                            message_id: None,
                            skipped: false,
                        });
                    }
                }
                Err(error) => tracing::warn!("rascunho fast-first falhou: {error}"),
            }
        }

        // 2) Passe completo, com deltas de streaming (texto acumulado).
        let system = assistant_prompts::suggestion_system_prompt(&mode, language);
        let completion_request = CompletionRequest::user(user)
            .with_system(system)
            .with_temperature(temperature)
            .with_max_tokens(max_tokens);
        let session_id = session.id.clone();
        let mode_id = mode.id.clone();
        let mode_name = mode.name.clone();
        let mode_icon = mode.icon.clone();
        let mode_color = mode.color.clone();
        let until_ms = recent.until_ms;
        let mut emit = |partial: &str| {
            // O `Subject:` é o título gerado pelo agente (BR-038): sai do corpo.
            let (title, text) = prompts::parse_suggestion_response(partial);
            if !text.is_empty() {
                self.publisher.emit_suggestion(&Suggestion {
                    session_id: session_id.clone(),
                    mode_id: mode_id.clone(),
                    mode_name: mode_name.clone(),
                    mode_icon: mode_icon.clone(),
                    mode_color: mode_color.clone(),
                    phase: SuggestionPhase::Full,
                    title: title.unwrap_or_default(),
                    text,
                    at_ms: now_rel,
                    context_until_ms: until_ms,
                    message_id: None,
                    skipped: false,
                });
            }
        };
        let completion = self
            .llm
            .complete_streaming(&completion_request, &mut emit)?;
        let (title, text) = prompts::parse_suggestion_response(&completion.text);
        if text.is_empty() {
            return Err(GhostWhisperError::Other(
                "o modelo devolveu uma sugestão vazia".to_string(),
            ));
        }

        let message_id = if request.persist {
            Some(self.persist(&session.id, now_rel, &text)?)
        } else {
            None
        };

        let suggestion = Suggestion {
            session_id: session.id.clone(),
            mode_id: mode.id,
            mode_name: mode.name,
            mode_icon: mode.icon,
            mode_color: mode.color,
            phase: SuggestionPhase::Full,
            title: title.unwrap_or_default(),
            text,
            at_ms: now_rel,
            context_until_ms: recent.until_ms,
            message_id,
            skipped: false,
        };
        self.remember(&session.id, &suggestion);
        Ok(suggestion)
    }

    fn pin(&self, request: PinSuggestionRequest) -> Result<Suggestion> {
        let settings = self.settings.load()?;
        let session = self.sessions.get(&request.session_id)?.ok_or_else(|| {
            GhostWhisperError::Config(format!("sessão {} não encontrada", request.session_id))
        })?;
        let mode =
            assistant_prompts::resolve_mode(&request.mode_id, &settings.custom_assistant_modes);
        let mode_name = mode
            .as_ref()
            .map(|mode| mode.name.clone())
            .unwrap_or_else(|| request.mode_id.clone());
        // Identidade do agente no momento da fixação (BR-037).
        let (mode_icon, mode_color) =
            mode.map(|mode| (mode.icon, mode.color)).unwrap_or_else(|| {
                (
                    crate::domain::assistant::default_agent_icon(),
                    crate::domain::assistant::default_agent_color(),
                )
            });

        let text = request.text.trim();
        if text.is_empty() {
            return Err(GhostWhisperError::Config(
                "sugestão vazia: nada para fixar".to_string(),
            ));
        }
        let at_ms = request.at_ms.max(0);
        let message_id = self.persist(&session.id, at_ms, text)?;

        Ok(Suggestion {
            session_id: session.id,
            mode_id: request.mode_id,
            mode_name,
            mode_icon,
            mode_color,
            phase: SuggestionPhase::Full,
            title: String::new(),
            text: text.to_string(),
            at_ms,
            context_until_ms: at_ms,
            message_id: Some(message_id),
            skipped: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::memory::in_memory_messages_repository::InMemoryMessagesRepository;
    use crate::adapters::persistence::memory::in_memory_session_repository::InMemorySessionRepository;
    use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::adapters::system::system_clock::FixedClock;
    use crate::application::test_support::FakeLlmController;
    use crate::domain::audio::AudioLevel;
    use crate::domain::session::{Session, SessionStatus};
    use crate::domain::settings::Settings;
    use crate::domain::transcription::{Speaker, TranscriptSegment};
    use std::sync::Mutex;

    use crate::ports::event_publisher::EventPublisher;
    use crate::ports::repositories::{MessagesRepository, TranscriptionsRepository};
    use crate::ports::session_controller::AudioChannel;

    fn segment(id: &str, at: i64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            session_id: "s1".to_string(),
            speaker: Speaker::They,
            text: text.to_string(),
            start_ms: at,
            end_ms: at + 500,
            is_final: true,
        }
    }

    fn session() -> Session {
        Session {
            id: "s1".to_string(),
            started_at_ms: 1_000,
            ended_at_ms: None,
            provider: "local".to_string(),
            model: None,
            transcription_model: None,
            user_id: None,
            subject: None,
            overview: None,
            status: SessionStatus::Active,
        }
    }

    /// Publicador que registra as sugestões efêmeras emitidas.
    struct RecordingPublisher {
        suggestions: Mutex<Vec<Suggestion>>,
    }
    impl RecordingPublisher {
        fn new() -> Self {
            Self {
                suggestions: Mutex::new(Vec::new()),
            }
        }
        fn texts(&self) -> Vec<String> {
            self.suggestions
                .lock()
                .expect("suggestions")
                .iter()
                .map(|suggestion| suggestion.text.clone())
                .collect()
        }
        /// Fases publicadas, na ordem (BR-038).
        fn phases(&self) -> Vec<SuggestionPhase> {
            self.suggestions
                .lock()
                .expect("suggestions")
                .iter()
                .map(|suggestion| suggestion.phase)
                .collect()
        }
        /// Sugestões publicadas, na ordem.
        fn all(&self) -> Vec<Suggestion> {
            self.suggestions.lock().expect("suggestions").clone()
        }
    }
    impl EventPublisher for RecordingPublisher {
        fn emit_segment(&self, _segment: &TranscriptSegment) {}
        fn emit_audio_level(&self, _channel: AudioChannel, _level: AudioLevel) {}
        fn emit_session_state(&self, _state: &crate::ports::event_publisher::SessionStateEvent) {}
        fn emit_insight(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion(&self, suggestion: &Suggestion) {
            self.suggestions
                .lock()
                .expect("suggestions")
                .push(suggestion.clone());
        }
        fn emit_suggestion_cadence(
            &self,
            _cadence: &crate::ports::event_publisher::SuggestionCadenceEvent,
        ) {
        }
        fn emit_asr_status(&self, _status: &crate::domain::transcription::AsrStatus) {}
    }

    struct Fixture {
        service: SuggestionService,
        llm: Arc<FakeLlmController>,
        transcriptions: Arc<InMemoryTranscriptionsRepository>,
        messages: Arc<InMemoryMessagesRepository>,
        publisher: Arc<RecordingPublisher>,
    }

    fn fixture(settings: Settings, response: &str) -> Fixture {
        let sessions = Arc::new(InMemorySessionRepository::new());
        sessions.create(&session()).expect("create session");
        let transcriptions = Arc::new(InMemoryTranscriptionsRepository::new());
        let messages = Arc::new(InMemoryMessagesRepository::new());
        let llm = Arc::new(FakeLlmController::new().with_response(response));
        let publisher = Arc::new(RecordingPublisher::new());
        let service = SuggestionService::new(
            llm.clone(),
            Arc::new(InMemorySettingsStore::with_settings(settings)),
            sessions,
            transcriptions.clone(),
            messages.clone(),
            Arc::new(FixedClock(65_000)),
            publisher.clone(),
        );
        Fixture {
            service,
            llm,
            transcriptions,
            messages,
            publisher,
        }
    }

    #[test]
    fn generates_ephemeral_by_default() {
        let fixture = fixture(Settings::default(), "## Easy\n\né uma fila.");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "o que é QPS?"))
            .unwrap();

        let suggestion = fixture
            .service
            .generate(
                SuggestionRequest::ephemeral(None, "meeting".to_string())
                    .with_last_suggestion(Some("sugestão anterior".to_string())),
            )
            .expect("generate");

        assert!(!suggestion.skipped);
        assert_eq!(suggestion.mode_id, "meeting");
        assert_eq!(suggestion.mode_name, "Reunião");
        assert!(suggestion.text.contains("fila"));
        assert!(suggestion.message_id.is_none(), "efêmera não grava");
        assert!(fixture.messages.list_by_session("s1").unwrap().is_empty());

        let request = fixture.llm.last_request().expect("pedido");
        assert!(request.prompt.contains("o que é QPS?"));
        assert!(request.prompt.contains("<last_suggestion>"));
        assert!(request.prompt.contains("sugestão anterior"));
        assert!(request.system.unwrap().contains("live meeting copilot"));
    }

    #[test]
    fn fast_first_emits_a_short_draft_before_the_full_suggestion() {
        // BR-028: o rascunho (fast-first) é publicado antes do texto completo.
        let fixture = fixture(Settings::default(), "> peça o prazo");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "e o prazo?"))
            .unwrap();

        fixture
            .service
            .generate(
                SuggestionRequest::ephemeral(None, "meeting".to_string()).with_fast_first(true),
            )
            .expect("generate");

        let requests = fixture.llm.requests();
        assert_eq!(requests.len(), 2, "rascunho + passe completo");
        assert_eq!(requests[0].max_tokens, Some(FAST_FIRST_MAX_TOKENS));
        assert_eq!(requests[0].temperature, Some(FAST_FIRST_TEMPERATURE));
        assert!(requests[0]
            .system
            .as_deref()
            .unwrap_or_default()
            .contains("ONLY the first useful suggestion"));
        assert_eq!(requests[1].max_tokens, Some(MEETING_MAX_TOKENS));

        // Os dois passes chegam à UI (rascunho + acumulado final).
        let texts = fixture.publisher.texts();
        assert_eq!(texts, vec!["> peça o prazo", "> peça o prazo"]);
        // Fases distintas (BR-038): a rápida não substitui a completa.
        assert_eq!(
            fixture.publisher.phases(),
            vec![SuggestionPhase::Quick, SuggestionPhase::Full]
        );
    }

    #[test]
    fn does_not_generate_again_without_new_speech() {
        // BR-039: sem fala nova, não há nova sugestão (e nenhuma chamada ao LLM).
        let fixture = fixture(Settings::default(), "> uma linha");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "fala"))
            .unwrap();

        let first = fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .expect("primeira");
        assert!(!first.skipped);
        assert_eq!(fixture.llm.call_count(), 1);

        let second = fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .expect("segunda");
        assert_eq!(fixture.llm.call_count(), 1, "não chama o LLM de novo");
        assert_eq!(second.text, first.text, "reaproveita a última sugestão");
        assert!(!second.skipped);
    }

    #[test]
    fn a_stale_request_cursor_does_not_duplicate_the_suggestion() {
        // A janela gerou (cache no cursor novo); o agendador ainda manda o
        // cursor velho. Antes, a mesma fala gerava **outra** sugestão (a
        // duplicata do histórico). Agora vale o cursor mais avançado.
        let fixture = fixture(Settings::default(), "> uma linha");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "fala"))
            .unwrap();

        let first = fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .expect("primeira");
        assert_eq!(fixture.llm.call_count(), 1);

        let second = fixture
            .service
            .generate(
                SuggestionRequest::ephemeral(None, "meeting".to_string())
                    // O cursor do agendador ficou para trás (0).
                    .with_since_ms(Some(0)),
            )
            .expect("segunda");
        assert_eq!(fixture.llm.call_count(), 1, "não chama o LLM de novo");
        assert_eq!(second.at_ms, first.at_ms, "mesma sugestão, sem duplicata");
    }

    #[test]
    fn force_generates_even_without_new_speech() {
        // O botão "Sugerir agora" sempre gera.
        let fixture = fixture(Settings::default(), "> uma linha");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "fala"))
            .unwrap();

        fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .expect("primeira");
        let forced = fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()).with_force(true))
            .expect("forçada");
        assert_eq!(fixture.llm.call_count(), 2);
        assert!(!forced.skipped);
    }

    #[test]
    fn new_speech_after_the_cursor_generates_again() {
        let fixture = fixture(Settings::default(), "> uma linha");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "fala"))
            .unwrap();

        fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .expect("primeira");
        fixture
            .transcriptions
            .insert(&segment("2", 70_000, "mais fala"))
            .unwrap();
        fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .expect("segunda");
        assert_eq!(fixture.llm.call_count(), 2, "fala nova gera de novo");
    }

    #[test]
    fn parses_the_ai_title_and_strips_it_from_the_body() {
        // BR-038: o `Subject:` do agente vira o título e sai do Markdown.
        let fixture = fixture(
            Settings::default(),
            "Subject: Como inverter uma árvore binária em Python\n\n## Easy\n\nEu faria assim.",
        );
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "como inverto uma árvore?"))
            .unwrap();

        let suggestion = fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .expect("generate");

        assert_eq!(
            suggestion.title,
            "Como inverter uma árvore binária em Python"
        );
        assert!(!suggestion.text.contains("Subject:"));
        assert!(suggestion.text.contains("## Easy"));

        // O evento de streaming também leva o título.
        assert!(fixture
            .publisher
            .all()
            .iter()
            .any(|item| item.title == "Como inverter uma árvore binária em Python"));
    }

    #[test]
    fn publishes_the_agent_identity_snapshot_with_the_suggestion() {
        // BR-037: o histórico precisa mostrar quem respondeu, mesmo que o agente
        // atual seja outro.
        let fixture = fixture(Settings::default(), "> fala");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "e o prazo?"))
            .unwrap();

        fixture
            .service
            .generate(
                SuggestionRequest::ephemeral(None, "leetcode".to_string()).with_fast_first(true),
            )
            .expect("generate");

        let published = fixture.publisher.all();
        assert!(!published.is_empty());
        for suggestion in published {
            assert_eq!(suggestion.mode_icon, "code");
            assert_eq!(suggestion.mode_color, "emerald");
            assert_eq!(suggestion.mode_id, "leetcode");
        }
    }

    #[test]
    fn without_fast_first_only_the_full_pass_runs() {
        let fixture = fixture(Settings::default(), "> uma linha");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "fala"))
            .unwrap();

        fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .expect("generate");

        assert_eq!(fixture.llm.call_count(), 1);
        assert_eq!(fixture.publisher.texts(), vec!["> uma linha"]);
        assert_eq!(fixture.publisher.phases(), vec![SuggestionPhase::Full]);
    }

    #[test]
    fn streams_the_accumulated_text_to_the_ui() {
        use crate::domain::llm::{Completion, CompletionRequest};
        use crate::ports::llm_controller::LlmController;

        /// Provider que emite o texto em pedaços (como o SSE real).
        struct ChunkedLlm;
        impl LlmController for ChunkedLlm {
            fn provider_label(&self) -> crate::error::Result<String> {
                Ok("chunked".into())
            }
            fn validate(&self) -> crate::error::Result<String> {
                Ok("ok".into())
            }
            fn complete(&self, _request: &CompletionRequest) -> crate::error::Result<Completion> {
                Ok(Completion {
                    text: "> completo".into(),
                    model: "chunked".into(),
                })
            }
            fn complete_streaming(
                &self,
                _request: &CompletionRequest,
                on_text: &mut dyn FnMut(&str),
            ) -> crate::error::Result<Completion> {
                on_text("> olá");
                on_text("> olá, tudo bem?");
                Ok(Completion {
                    text: "> olá, tudo bem?".into(),
                    model: "chunked".into(),
                })
            }
        }

        let sessions = Arc::new(InMemorySessionRepository::new());
        sessions.create(&session()).expect("create session");
        let transcriptions = Arc::new(InMemoryTranscriptionsRepository::new());
        transcriptions
            .insert(&segment("1", 64_000, "oi, tudo bem?"))
            .unwrap();
        let publisher = Arc::new(RecordingPublisher::new());
        let service = SuggestionService::new(
            Arc::new(ChunkedLlm),
            Arc::new(InMemorySettingsStore::with_settings(Settings::default())),
            sessions,
            transcriptions,
            Arc::new(InMemoryMessagesRepository::new()),
            Arc::new(FixedClock(65_000)),
            publisher.clone(),
        );

        let suggestion = service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .expect("generate");

        assert_eq!(suggestion.text, "> olá, tudo bem?");
        assert_eq!(
            publisher.texts(),
            vec!["> olá", "> olá, tudo bem?"],
            "cada delta vira uma atualização da bolha"
        );
    }

    #[test]
    fn persists_when_asked_and_uses_the_session_clock() {
        let fixture = fixture(Settings::default(), "> diga que concorda.");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "podemos fechar?"))
            .unwrap();

        let suggestion = fixture
            .service
            .generate(SuggestionRequest::persist(None))
            .expect("generate");

        assert_eq!(suggestion.at_ms, 64_000);
        assert_eq!(suggestion.message_id.as_deref().map(str::len), Some(36));
        let stored = fixture.messages.list_by_session("s1").unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].content, "> diga que concorda.");
        assert_eq!(stored[0].timestamp_ms, 64_000);
    }

    #[test]
    fn skips_without_new_speech() {
        let fixture = fixture(Settings::default(), "x");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "fala"))
            .unwrap();

        let suggestion = fixture
            .service
            .generate(
                SuggestionRequest::ephemeral(None, "meeting".to_string())
                    .with_since_ms(Some(64_000)),
            )
            .expect("skip");

        assert!(suggestion.skipped);
        assert!(suggestion.text.is_empty());
        assert_eq!(fixture.llm.call_count(), 0, "não chamou o LLM");
    }

    #[test]
    fn uses_the_selected_custom_mode_and_extra_context() {
        use crate::domain::assistant::AssistantMode;

        let settings = Settings {
            assistant_mode_id: "custom-vendas".into(),
            custom_assistant_modes: vec![AssistantMode {
                id: "custom-vendas".into(),
                name: "Vendas".into(),
                category: "custom".into(),
                system_prompt: "Você ajuda em vendas B2B.".into(),
                suggestion_prompt: "Sugira a próxima fala de vendas.".into(),
                context: "Produto: GhostWhisper.".into(),
                icon: "briefcase".into(),
                color: "coral".into(),
            }],
            ..Settings::default()
        };
        let fixture = fixture(settings, "> fale sobre o retorno.");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "está caro"))
            .unwrap();

        let suggestion = fixture
            .service
            .generate(
                SuggestionRequest::ephemeral(None, "custom-vendas".to_string())
                    .with_extra_context(Some("Tabela de preços: 10 reais.".to_string())),
            )
            .expect("generate");

        assert_eq!(suggestion.mode_name, "Vendas");
        let request = fixture.llm.last_request().expect("pedido");
        let system = request.system.unwrap();
        assert!(system.contains("Sugira a próxima fala de vendas."));
        assert!(system.contains("Você ajuda em vendas B2B."));
        assert!(system.contains("Produto: GhostWhisper."));
        assert!(request.prompt.contains("Tabela de preços"));
    }

    #[test]
    fn technical_modes_use_longer_output() {
        let fixture = fixture(Settings::default(), "**Big O:** O(n) time | O(1) space");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "resolva two sum"))
            .unwrap();

        fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "leetcode".to_string()))
            .expect("generate");

        let request = fixture.llm.last_request().expect("pedido");
        assert_eq!(request.max_tokens, Some(TECHNICAL_MAX_TOKENS));
        assert!(request.system.unwrap().contains("LeetCode"));
    }

    #[test]
    fn pin_persists_without_calling_the_llm() {
        let fixture = fixture(Settings::default(), "x");
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "fala"))
            .unwrap();

        let pinned = fixture
            .service
            .pin(PinSuggestionRequest {
                session_id: "s1".to_string(),
                mode_id: "leetcode".to_string(),
                text: "## Solução\n\n```rust\nfn main() {}\n```".to_string(),
                at_ms: 60_000,
            })
            .expect("pin");

        assert_eq!(pinned.mode_name, "LeetCode");
        assert_eq!(pinned.at_ms, 60_000);
        assert!(pinned.message_id.is_some());
        assert_eq!(fixture.llm.call_count(), 0, "pin não chama o LLM");
        assert_eq!(fixture.messages.list_by_session("s1").unwrap().len(), 1);
    }

    #[test]
    fn errors_without_speech_session_or_mode() {
        let fixture = fixture(Settings::default(), "x");
        assert!(fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .is_err());

        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "fala"))
            .unwrap();
        assert!(fixture
            .service
            .generate(SuggestionRequest::ephemeral(
                Some("nao-existe".to_string()),
                "meeting".to_string(),
            ))
            .is_err());
        assert!(fixture
            .service
            .generate(SuggestionRequest::ephemeral(None, "nao-existe".to_string()))
            .is_err());
    }

    #[test]
    fn recent_context_ignores_old_speech() {
        let fixture = fixture(Settings::default(), "> ok");
        fixture
            .transcriptions
            .insert(&segment("1", 1_000, "fala antiga"))
            .unwrap();
        fixture
            .transcriptions
            .insert(&segment("2", 64_000, "fala recente"))
            .unwrap();

        let service = fixture.service.with_limits(SuggestionLimits {
            window_ms: 5_000,
            ..SuggestionLimits::default()
        });
        service
            .generate(SuggestionRequest::ephemeral(None, "meeting".to_string()))
            .expect("generate");

        let request = fixture.llm.last_request().expect("pedido");
        assert!(request.prompt.contains("fala recente"));
        assert!(!request.prompt.contains("fala antiga"));
    }
}
