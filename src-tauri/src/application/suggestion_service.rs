//! Caso de uso: sugestão contextual por agente.
//!
//! A sugestão pode ser **efêmera** (janela de sugestões) ou **persistida**
//! (botão “Sugerir”, atalho global e “Fixar”). O contexto são os últimos
//! momentos da conversa; um `since_ms` evita chamar o LLM sem fala nova.

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::llm::CompletionRequest;
use crate::domain::message::{ConversationMessage, MessageRole};
use crate::error::{GhostWhisperError, Result};
use crate::ports::clock::Clock;
use crate::ports::llm_controller::LlmController;
use crate::ports::repositories::{MessagesRepository, SessionRepository, TranscriptionsRepository};
use crate::ports::settings_store::SettingsStore;
use crate::ports::suggestion_generator::{
    PinSuggestionRequest, Suggestion, SuggestionGenerator, SuggestionRequest,
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

/// Gera sugestões contextuais por agente.
pub struct SuggestionService {
    llm: Arc<dyn LlmController>,
    settings: Arc<dyn SettingsStore>,
    sessions: Arc<dyn SessionRepository>,
    transcriptions: Arc<dyn TranscriptionsRepository>,
    messages: Arc<dyn MessagesRepository>,
    clock: Arc<dyn Clock>,
    limits: SuggestionLimits,
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
    ) -> Self {
        Self {
            llm,
            settings,
            sessions,
            transcriptions,
            messages,
            clock,
            limits: SuggestionLimits::default(),
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
        let settings = self.settings.load()?;
        let session = resolve_session(self.sessions.as_ref(), request.session_id)?;
        let mode = self.resolve_mode(request.mode_id.as_deref(), &settings)?;
        let segments = self.transcriptions.list_by_session(&session.id)?;
        let now_rel = (self.clock.now_ms() - session.started_at_ms).max(0);

        let recent = context::recent_context(
            &segments,
            now_rel,
            self.limits.window_ms,
            self.limits.max_segments,
            self.limits.max_context_chars,
        )
        .ok_or_else(|| {
            GhostWhisperError::Config("transcrição vazia: nada para sugerir".to_string())
        })?;

        if let Some(since) = request.since_ms {
            if recent.until_ms <= since {
                return Ok(Suggestion {
                    session_id: session.id.clone(),
                    mode_id: mode.id,
                    mode_name: mode.name,
                    text: String::new(),
                    at_ms: since,
                    context_until_ms: recent.until_ms,
                    message_id: None,
                    skipped: true,
                });
            }
        }

        let last = Self::clamp(request.last_suggestion, LAST_SUGGESTION_MAX_CHARS);
        let extra = Self::clamp(request.extra_context, EXTRA_CONTEXT_MAX_CHARS);
        let language = Some(settings.source_language.as_str());
        let system = assistant_prompts::suggestion_system_prompt(&mode, language);
        let user = assistant_prompts::suggestion_user_prompt(
            &recent.text,
            last.as_deref(),
            extra.as_deref(),
        );
        let (max_tokens, temperature) = sampling(&mode);

        let completion = self.llm.complete(
            &CompletionRequest::user(user)
                .with_system(system)
                .with_temperature(temperature)
                .with_max_tokens(max_tokens),
        )?;
        let text = prompts::normalize_markdown(&completion.text);
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

        Ok(Suggestion {
            session_id: session.id,
            mode_id: mode.id,
            mode_name: mode.name,
            text,
            at_ms: now_rel,
            context_until_ms: recent.until_ms,
            message_id,
            skipped: false,
        })
    }

    fn pin(&self, request: PinSuggestionRequest) -> Result<Suggestion> {
        let settings = self.settings.load()?;
        let session = self.sessions.get(&request.session_id)?.ok_or_else(|| {
            GhostWhisperError::Config(format!("sessão {} não encontrada", request.session_id))
        })?;
        let mode_name =
            assistant_prompts::resolve_mode(&request.mode_id, &settings.custom_assistant_modes)
                .map(|mode| mode.name)
                .unwrap_or_else(|| request.mode_id.clone());

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
    use crate::domain::session::{Session, SessionStatus};
    use crate::domain::settings::Settings;
    use crate::domain::transcription::{Speaker, TranscriptSegment};
    use crate::ports::repositories::{MessagesRepository, TranscriptionsRepository};

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

    struct Fixture {
        service: SuggestionService,
        llm: Arc<FakeLlmController>,
        transcriptions: Arc<InMemoryTranscriptionsRepository>,
        messages: Arc<InMemoryMessagesRepository>,
    }

    fn fixture(settings: Settings, response: &str) -> Fixture {
        let sessions = Arc::new(InMemorySessionRepository::new());
        sessions.create(&session()).expect("create session");
        let transcriptions = Arc::new(InMemoryTranscriptionsRepository::new());
        let messages = Arc::new(InMemoryMessagesRepository::new());
        let llm = Arc::new(FakeLlmController::new().with_response(response));
        let service = SuggestionService::new(
            llm.clone(),
            Arc::new(InMemorySettingsStore::with_settings(settings)),
            sessions,
            transcriptions.clone(),
            messages.clone(),
            Arc::new(FixedClock(65_000)),
        );
        Fixture {
            service,
            llm,
            transcriptions,
            messages,
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
