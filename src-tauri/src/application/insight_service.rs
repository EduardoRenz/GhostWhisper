//! Caso de uso: gerar sugestões/insights a partir da transcrição recente.

use std::sync::Arc;

use crate::domain::llm::CompletionRequest;
use crate::domain::transcription::TranscriptSegment;
use crate::error::{GhostWhisperError, Result};
use crate::ports::insight_generator::InsightGenerator;
use crate::ports::llm_controller::LlmController;
use crate::ports::repositories::{SessionRepository, TranscriptionsRepository};
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
}

impl InsightService {
    /// Cria o serviço com as portas resolvidas.
    pub fn new(
        llm: Arc<dyn LlmController>,
        settings: Arc<dyn SettingsStore>,
        sessions: Arc<dyn SessionRepository>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
    ) -> Self {
        Self {
            llm,
            settings,
            sessions,
            transcriptions,
        }
    }

    fn resolve_session(&self, session_id: Option<String>) -> Result<String> {
        if let Some(id) = session_id {
            return Ok(id);
        }
        self.sessions
            .list_recent(1)?
            .into_iter()
            .next()
            .map(|session| session.id)
            .ok_or_else(|| GhostWhisperError::Config("nenhuma sessão disponível".to_string()))
    }
}

impl InsightGenerator for InsightService {
    fn generate(&self, session_id: Option<String>) -> Result<String> {
        let settings = self.settings.load()?;
        let session_id = self.resolve_session(session_id)?;
        let segments = self.transcriptions.list_by_session(&session_id)?;
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

        Ok(self.llm.complete(&request)?.text)
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
    use crate::adapters::persistence::memory::in_memory_session_repository::InMemorySessionRepository;
    use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;
    use crate::adapters::settings::memory::InMemorySettingsStore;
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

    fn service(
        with_session: bool,
    ) -> (
        InsightService,
        Arc<FakeLlmController>,
        Arc<InMemoryTranscriptionsRepository>,
    ) {
        let sessions = Arc::new(InMemorySessionRepository::new());
        let transcriptions = Arc::new(InMemoryTranscriptionsRepository::new());
        if with_session {
            sessions
                .create(&Session {
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
                })
                .expect("create session");
        }
        let llm = Arc::new(FakeLlmController::new().with_response("sugestão"));
        let service = InsightService::new(
            llm.clone(),
            Arc::new(InMemorySettingsStore::new()),
            sessions,
            transcriptions.clone(),
        );
        (service, llm, transcriptions)
    }

    #[test]
    fn generates_from_recent_session_context() {
        let (service, llm, transcriptions) = service(true);
        transcriptions
            .insert(&segment(
                "1",
                "s1",
                Speaker::They,
                "podemos fechar assim?",
                0,
            ))
            .unwrap();

        assert_eq!(service.generate(None).expect("generate"), "sugestão");

        let request = llm.last_request().expect("request");
        assert!(request.prompt.contains("podemos fechar assim?"));
        assert!(request.prompt.contains("THEY"));
    }

    #[test]
    fn errors_without_session() {
        let (service, _llm, _t) = service(false);
        assert!(service.generate(None).is_err());
    }

    #[test]
    fn errors_with_empty_transcription() {
        let (service, _llm, _t) = service(true);
        let error = service.generate(None).unwrap_err().to_string();
        assert!(error.contains("vazia"));
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
