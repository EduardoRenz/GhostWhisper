//! Caso de uso: gerar sugestões/insights a partir da transcrição recente.
//!
//! A sugestão é **persistida** como mensagem da sessão (`messages`, papel
//! `assistant`) com o tempo desde o início da sessão — é assim que ela entra na
//! linha temporal do histórico e do export Markdown (task 17/18).

use std::sync::Arc;

use uuid::Uuid;

use crate::domain::llm::CompletionRequest;
use crate::domain::message::{ConversationMessage, MessageRole};
use crate::domain::transcription::TranscriptSegment;
use crate::error::{GhostWhisperError, Result};
use crate::ports::clock::Clock;
use crate::ports::insight_generator::{InsightGenerator, InsightSuggestion};
use crate::ports::llm_controller::LlmController;
use crate::ports::repositories::{MessagesRepository, SessionRepository, TranscriptionsRepository};
use crate::ports::settings_store::SettingsStore;

use super::prompts;

/// Limite de caracteres do contexto enviado ao provider (controla custo).
const MAX_CONTEXT_CHARS: usize = 4_000;

/// Gera insights contextuais usando o provider configurado.
pub struct InsightService {
    llm: Arc<dyn LlmController>,
    settings: Arc<dyn SettingsStore>,
    sessions: Arc<dyn SessionRepository>,
    transcriptions: Arc<dyn TranscriptionsRepository>,
    messages: Arc<dyn MessagesRepository>,
    clock: Arc<dyn Clock>,
}

impl InsightService {
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
        }
    }

    /// Sessão alvo da sugestão (a pedida ou a mais recente).
    fn resolve_session(
        &self,
        session_id: Option<String>,
    ) -> Result<crate::domain::session::Session> {
        match session_id {
            Some(id) => self
                .sessions
                .get(&id)?
                .ok_or_else(|| GhostWhisperError::Config(format!("sessão {id} não encontrada"))),
            None => self
                .sessions
                .list_recent(1)?
                .into_iter()
                .next()
                .ok_or_else(|| GhostWhisperError::Config("nenhuma sessão disponível".to_string())),
        }
    }
}

impl InsightGenerator for InsightService {
    fn generate(&self, session_id: Option<String>) -> Result<InsightSuggestion> {
        let settings = self.settings.load()?;
        let session = self.resolve_session(session_id)?;
        let segments = self.transcriptions.list_by_session(&session.id)?;
        let context = build_context(&segments, MAX_CONTEXT_CHARS);
        if context.trim().is_empty() {
            return Err(GhostWhisperError::Config(
                "transcrição vazia: nada para sugerir".to_string(),
            ));
        }

        let system =
            prompts::insight_system_prompt(&settings.source_language, &settings.target_language);
        let request = CompletionRequest::user(prompts::insight_user_prompt(&context))
            .with_system(system)
            .with_temperature(0.3)
            .with_max_tokens(400);

        let text = self.llm.complete(&request)?.text;

        // Tempo desde o início da **sessão** (mesma escala das falas), para o
        // item entrar na linha temporal no lugar certo (BR-019).
        let at_ms = (self.clock.now_ms() - session.started_at_ms).max(0);
        let suggestion = InsightSuggestion {
            message_id: Uuid::new_v4().to_string(),
            session_id: session.id.clone(),
            text: text.clone(),
            at_ms,
        };

        self.messages.insert(&ConversationMessage {
            id: suggestion.message_id.clone(),
            session_id: session.id,
            role: MessageRole::Assistant,
            content: text,
            timestamp_ms: at_ms,
        })?;

        Ok(suggestion)
    }
}

/// Monta o contexto com os segmentos mais recentes, limitado a `max_chars`.
fn build_context(segments: &[TranscriptSegment], max_chars: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    for segment in segments.iter().rev() {
        let line = format!("{}: {}", segment.speaker.as_str(), segment.text);
        let line_len = line.chars().count();
        if total + line_len > max_chars && !lines.is_empty() {
            break;
        }
        total += line_len;
        lines.push(line);
    }
    lines.reverse();
    lines.join("\n")
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
    use crate::domain::transcription::Speaker;

    fn segment(
        id: &str,
        session_id: &str,
        speaker: Speaker,
        text: &str,
        at: i64,
    ) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            session_id: session_id.to_string(),
            speaker,
            text: text.to_string(),
            start_ms: at,
            end_ms: at + 500,
            is_final: true,
        }
    }

    fn session(id: &str, started_at_ms: i64) -> Session {
        Session {
            id: id.to_string(),
            started_at_ms,
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

    /// `now_ms = 65_000`, sessão começando em `1_000` ⇒ sugestão em `64_000`.
    struct Fixture {
        service: InsightService,
        llm: Arc<FakeLlmController>,
        transcriptions: Arc<InMemoryTranscriptionsRepository>,
        messages: Arc<InMemoryMessagesRepository>,
    }

    fn service(with_session: bool) -> Fixture {
        let sessions = Arc::new(InMemorySessionRepository::new());
        let transcriptions = Arc::new(InMemoryTranscriptionsRepository::new());
        let messages = Arc::new(InMemoryMessagesRepository::new());
        if with_session {
            sessions
                .create(&session("s1", 1_000))
                .expect("create session");
        }
        let llm = Arc::new(FakeLlmController::new().with_response("sugestão"));
        let service = InsightService::new(
            llm.clone(),
            Arc::new(InMemorySettingsStore::new()),
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
    fn generates_from_recent_session_context() {
        let fixture = service(true);
        fixture
            .transcriptions
            .insert(&segment(
                "1",
                "s1",
                Speaker::They,
                "podemos fechar assim?",
                0,
            ))
            .unwrap();

        let suggestion = fixture.service.generate(None).expect("generate");

        assert_eq!(suggestion.text, "sugestão");
        assert_eq!(suggestion.session_id, "s1");
        assert!(!suggestion.message_id.is_empty());

        let request = fixture.llm.last_request().expect("request");
        assert!(request.prompt.contains("podemos fechar assim?"));
        assert!(request.prompt.contains("THEY"));
    }

    #[test]
    fn persists_the_suggestion_with_the_session_clock() {
        let fixture = service(true);
        fixture
            .transcriptions
            .insert(&segment("1", "s1", Speaker::They, "e agora?", 0))
            .unwrap();

        let suggestion = fixture
            .service
            .generate(Some("s1".to_string()))
            .expect("generate");

        // 65.000 (agora) − 1.000 (início da sessão).
        assert_eq!(suggestion.at_ms, 64_000);

        let stored = fixture.messages.list_by_session("s1").expect("list");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, suggestion.message_id);
        assert_eq!(stored[0].role, MessageRole::Assistant);
        assert_eq!(stored[0].content, "sugestão");
        assert_eq!(stored[0].timestamp_ms, 64_000);
    }

    #[test]
    fn errors_without_session() {
        let fixture = service(false);
        assert!(fixture.service.generate(None).is_err());
    }

    #[test]
    fn errors_for_unknown_session() {
        let fixture = service(true);
        let error = fixture
            .service
            .generate(Some("nao-existe".to_string()))
            .unwrap_err()
            .to_string();
        assert!(error.contains("não encontrada"), "{error}");
    }

    #[test]
    fn errors_with_empty_transcription() {
        let fixture = service(true);
        let error = fixture.service.generate(None).unwrap_err().to_string();
        assert!(error.contains("vazia"));
        assert!(
            fixture
                .messages
                .list_by_session("s1")
                .expect("list")
                .is_empty(),
            "nada é gravado quando a geração falha"
        );
    }

    #[test]
    fn build_context_keeps_tail_and_limits_size() {
        let segments = vec![
            segment("1", "s1", Speaker::Me, "primeira fala longa", 0),
            segment("2", "s1", Speaker::They, "segunda fala", 1),
            segment("3", "s1", Speaker::Me, "terceira fala", 2),
        ];
        let context = build_context(&segments, 1000);
        assert!(context.contains("terceira fala"));
        assert!(context.contains("primeira fala"));

        // Limite pequeno: mantém só as falas mais recentes.
        let small = build_context(&segments, 20);
        assert!(small.contains("terceira fala"));
        assert!(!small.contains("primeira fala"));
    }
}
