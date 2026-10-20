//! Caso de uso: resumo incremental de uma sessão.
//!
//! Primeira geração usa o contrato completo do Perssua (adaptado); as
//! atualizações usam o cursor (`summarized_until_ms`) e o prompt incremental.
//! Sessões longas avançam em chunks (no máximo `max_chunks` por chamada) e a UI
//! retoma enquanto `has_more` for `true`.

use std::sync::Arc;

use crate::domain::ai_route::{AiCapability, AiRouteScope};
use crate::domain::llm::CompletionRequest;
use crate::domain::overview::MeetingSummary;
use crate::error::{GhostWhisperError, Result};
use crate::ports::clock::Clock;
use crate::ports::llm_controller::LlmController;
use crate::ports::meeting_summaries_repository::MeetingSummariesRepository;
use crate::ports::repositories::{SessionRepository, TranscriptionsRepository};
use crate::ports::settings_store::SettingsStore;
use crate::ports::summary_generator::{
    SummaryGenerator, SummaryMode, SummaryRequest, SummaryResult,
};

use super::context::{self, ContextChunk};
use super::prompts::summary as summary_prompts;
use super::session_lookup::resolve_session;

/// Limites do resumo (injetáveis nos testes).
#[derive(Debug, Clone, Copy)]
pub struct SummaryLimits {
    /// Caracteres de transcrição por chamada.
    pub max_context_chars: usize,
    /// Chunks processados por tick.
    pub max_chunks: usize,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for SummaryLimits {
    fn default() -> Self {
        Self {
            max_context_chars: 24_000,
            max_chunks: 3,
            max_tokens: 2_500,
            temperature: 0.2,
        }
    }
}

/// Gera e persiste o resumo da sessão.
pub struct SummaryService {
    llm: Arc<dyn LlmController>,
    settings: Arc<dyn SettingsStore>,
    sessions: Arc<dyn SessionRepository>,
    transcriptions: Arc<dyn TranscriptionsRepository>,
    summaries: Arc<dyn MeetingSummariesRepository>,
    clock: Arc<dyn Clock>,
    limits: SummaryLimits,
}

impl SummaryService {
    /// Cria o serviço com as portas resolvidas.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        llm: Arc<dyn LlmController>,
        settings: Arc<dyn SettingsStore>,
        sessions: Arc<dyn SessionRepository>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
        summaries: Arc<dyn MeetingSummariesRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            llm,
            settings,
            sessions,
            transcriptions,
            summaries,
            clock,
            limits: SummaryLimits::default(),
        }
    }

    /// Substitui os limites (testes de chunk).
    #[cfg(test)]
    pub fn with_limits(mut self, limits: SummaryLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Idioma pedido para o resumo (o `auto` vira detecção).
    fn language(&self, settings: &crate::domain::settings::Settings) -> Option<String> {
        Some(settings.source_language.clone())
    }

    /// Processa um chunk (primeiro usa o contrato completo; os demais, o incremental).
    #[allow(clippy::too_many_arguments)]
    fn process_chunk(
        &self,
        chunk: &ContextChunk,
        current: Option<&MeetingSummary>,
        language: Option<&str>,
    ) -> Result<(Option<String>, String)> {
        let request = match current {
            Some(previous) => CompletionRequest::user(summary_prompts::summary_update_user_prompt(
                &previous.markdown,
                &chunk.text,
                language,
            ))
            .with_system(summary_prompts::SUMMARY_UPDATE_SYSTEM_PROMPT)
            .with_temperature(self.limits.temperature)
            .with_max_tokens(self.limits.max_tokens),
            None => CompletionRequest::user(summary_prompts::summary_initial_user_prompt(
                &chunk.text,
                language,
            ))
            .with_system(summary_prompts::SUMMARY_SYSTEM_PROMPT)
            .with_temperature(self.limits.temperature)
            .with_max_tokens(self.limits.max_tokens),
        };

        let completion = self
            .llm
            .complete_for(&AiRouteScope::Capability(AiCapability::Summary), &request)?;
        let (subject, markdown) = summary_prompts::parse_summary_response(&completion.text);
        if markdown.trim().is_empty() {
            return Err(GhostWhisperError::Other(
                "o modelo devolveu um resumo vazio".to_string(),
            ));
        }
        Ok((subject, markdown))
    }
}

impl SummaryGenerator for SummaryService {
    fn get(&self, session_id: Option<String>) -> Result<Option<MeetingSummary>> {
        let session = resolve_session(self.sessions.as_ref(), session_id)?;
        self.summaries.get(&session.id)
    }

    fn generate(&self, request: SummaryRequest) -> Result<SummaryResult> {
        let settings = self.settings.load()?;
        let session = resolve_session(self.sessions.as_ref(), request.session_id)?;
        let segments = self.transcriptions.list_by_session(&session.id)?;
        let stored = self.summaries.get(&session.id)?;

        // `Manual` regenera do zero; `Auto` continua do cursor quando existe.
        let mut current = match request.mode {
            SummaryMode::Auto => stored.clone(),
            SummaryMode::Manual => None,
        };
        let cursor = current.as_ref().map(|summary| summary.summarized_until_ms);

        let chunks = context::chunks_after(
            &segments,
            cursor,
            self.limits.max_context_chars,
            self.limits.max_chunks,
        );

        if chunks.is_empty() {
            return match stored {
                Some(previous) if request.mode == SummaryMode::Auto => Ok(SummaryResult {
                    summary: previous,
                    updated: false,
                    has_more: false,
                }),
                _ => Err(GhostWhisperError::Config(
                    "transcrição vazia: nada para resumir".to_string(),
                )),
            };
        }

        let language = self.language(&settings);
        for chunk in &chunks {
            let (subject, markdown) =
                self.process_chunk(chunk, current.as_ref(), language.as_deref())?;
            let summarized_from_ms = current
                .as_ref()
                .map(|summary| summary.summarized_from_ms)
                .unwrap_or(chunk.from_ms);
            let subject =
                subject.or_else(|| current.as_ref().and_then(|summary| summary.subject.clone()));
            current = Some(MeetingSummary {
                session_id: session.id.clone(),
                subject,
                markdown,
                summarized_from_ms,
                summarized_until_ms: chunk.until_ms,
                updated_at_ms: self.clock.now_ms(),
            });
        }

        let summary =
            current.ok_or_else(|| GhostWhisperError::Other("nenhum resumo gerado".to_string()))?;

        // Há mais conteúdo? O próximo tick continua do cursor.
        let last_final = context::last_final_ms(&segments);
        let has_more = last_final
            .map(|last| last > summary.summarized_until_ms)
            .unwrap_or(false);

        self.summaries.save(&summary)?;
        self.sessions.set_overview(&session.id, &summary.markdown)?;
        if session.subject.is_none() {
            if let Some(subject) = summary.subject.as_deref().map(str::trim) {
                if !subject.is_empty() {
                    self.sessions.rename(&session.id, subject)?;
                }
            }
        }

        Ok(SummaryResult {
            summary,
            updated: true,
            has_more,
        })
    }
}

/// Helper exposto para testes que precisam montar o contexto como o serviço faz.
#[cfg(test)]
fn build_context_for_tests(
    segments: &[crate::domain::transcription::TranscriptSegment],
    cursor: Option<i64>,
    max_chars: usize,
) -> Vec<ContextChunk> {
    context::chunks_after(segments, cursor, max_chars, 3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::memory::in_memory_meeting_summaries_repository::InMemoryMeetingSummariesRepository;
    use crate::adapters::persistence::memory::in_memory_session_repository::InMemorySessionRepository;
    use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;
    use crate::adapters::settings::memory::InMemorySettingsStore;
    use crate::adapters::system::system_clock::FixedClock;
    use crate::application::test_support::FakeLlmController;
    use crate::domain::ai_route::{AiCapability, AiRouteScope};
    use crate::domain::session::{Session, SessionStatus};
    use crate::domain::transcription::{Speaker, TranscriptSegment};
    use crate::ports::meeting_summaries_repository::MeetingSummariesRepository;
    use crate::ports::repositories::TranscriptionsRepository;

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
        service: SummaryService,
        llm: Arc<FakeLlmController>,
        transcriptions: Arc<InMemoryTranscriptionsRepository>,
        summaries: Arc<InMemoryMeetingSummariesRepository>,
        sessions: Arc<InMemorySessionRepository>,
    }

    fn fixture(responses: Vec<&str>) -> Fixture {
        let sessions = Arc::new(InMemorySessionRepository::new());
        sessions.create(&session()).expect("create session");
        let transcriptions = Arc::new(InMemoryTranscriptionsRepository::new());
        let summaries = Arc::new(InMemoryMeetingSummariesRepository::new());
        let llm = Arc::new(
            FakeLlmController::new()
                .with_responses(responses.into_iter().map(str::to_string).collect()),
        );
        let service = SummaryService::new(
            llm.clone(),
            Arc::new(InMemorySettingsStore::new()),
            sessions.clone(),
            transcriptions.clone(),
            summaries.clone(),
            Arc::new(FixedClock(65_000)),
        );
        Fixture {
            service,
            llm,
            transcriptions,
            summaries,
            sessions,
        }
    }

    #[test]
    fn asks_the_llm_with_the_summary_capability() {
        // BR-048: o resumo pede a fonte da **capacidade** Summary.
        let fixture = fixture(vec![
            "SUBJECT: Planejamento\nCONTENT: ## Visão geral\n\nTudo certo.",
        ]);
        fixture
            .transcriptions
            .insert(&segment("1", 64_000, "vamos planejar"))
            .unwrap();

        fixture
            .service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .expect("summary");

        let scopes = fixture.llm.scopes();
        assert!(!scopes.is_empty(), "nenhum escopo foi pedido");
        for scope in scopes {
            assert_eq!(scope, AiRouteScope::Capability(AiCapability::Summary));
        }
    }

    #[test]
    fn generates_and_persists_the_first_summary() {
        let fixture = fixture(vec![
            "SUBJECT: Planejamento\nCONTENT: ## Visão geral\n\nTudo certo.",
        ]);
        fixture
            .transcriptions
            .insert(&segment("1", 0, "podemos fechar assim?"))
            .unwrap();

        let result = fixture
            .service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .expect("generate");

        assert!(result.updated);
        assert!(!result.has_more);
        assert_eq!(result.summary.subject.as_deref(), Some("Planejamento"));
        assert!(result.summary.markdown.contains("Visão geral"));
        assert_eq!(result.summary.summarized_from_ms, 0);
        assert_eq!(result.summary.summarized_until_ms, 0);

        // Persistido e espelhado na sessão.
        assert!(fixture.summaries.get("s1").unwrap().is_some());
        let stored = fixture.sessions.get("s1").unwrap().unwrap();
        assert_eq!(
            stored.overview.as_deref(),
            Some("## Visão geral\n\nTudo certo.")
        );
        assert_eq!(stored.subject.as_deref(), Some("Planejamento"));

        let request = fixture.llm.last_request().expect("pedido");
        assert!(request.prompt.contains("podemos fechar assim?"));
        assert!(request.system.unwrap().contains("Confirmed Decisions"));
    }

    #[test]
    fn skips_the_llm_when_there_is_nothing_new() {
        let fixture = fixture(vec!["SUBJECT: T\nCONTENT: ## A\n\nok"]);
        fixture
            .transcriptions
            .insert(&segment("1", 0, "fala"))
            .unwrap();
        let first = fixture
            .service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .expect("primeiro");
        assert_eq!(first.summary.summarized_until_ms, 0);

        let calls = fixture.llm.call_count();
        let second = fixture
            .service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .expect("segundo");

        assert!(!second.updated);
        assert_eq!(fixture.llm.call_count(), calls, "não chamou o LLM de novo");
        assert_eq!(second.summary.markdown, "## A\n\nok");
    }

    #[test]
    fn updates_incrementally_from_the_cursor() {
        let fixture = fixture(vec![
            "SUBJECT: T\nCONTENT: ## A\n\nprimeiro",
            "## A\n\nprimeiro\n\n## B\n\nsegundo",
        ]);
        fixture
            .transcriptions
            .insert(&segment("1", 0, "primeiro"))
            .unwrap();
        fixture
            .service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .expect("primeiro");

        fixture
            .transcriptions
            .insert(&segment("2", 5_000, "segundo"))
            .unwrap();
        let second = fixture
            .service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .expect("segundo");

        assert!(second.updated);
        assert_eq!(second.summary.summarized_until_ms, 5_000);
        assert!(second.summary.markdown.contains("segundo"));

        let request = fixture.llm.last_request().expect("pedido");
        assert!(request.prompt.contains("CURRENT SESSION SUMMARY"));
        assert!(request.prompt.contains("segundo"));
        assert!(request.prompt.contains("primeiro"));
    }

    #[test]
    fn manual_regenerates_from_scratch() {
        let fixture = fixture(vec![
            "SUBJECT: T\nCONTENT: ## A\n\nprimeiro",
            "SUBJECT: Novo\nCONTENT: ## A\n\nrefeito",
        ]);
        fixture
            .transcriptions
            .insert(&segment("1", 0, "primeiro"))
            .unwrap();
        fixture
            .service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .expect("auto");

        fixture
            .service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Manual,
            })
            .expect("manual");

        let request = fixture.llm.last_request().expect("pedido");
        assert!(
            request.prompt.contains("Conversation to summarize"),
            "manual usa o contrato completo"
        );
        assert!(request.system.unwrap().contains("Confirmed Decisions"));
    }

    #[test]
    fn long_sessions_advance_in_chunks_and_report_has_more() {
        let fixture = fixture(vec![
            "SUBJECT: T\nCONTENT: ## A\n\num",
            "## A\n\num e dois",
            "## A\n\num, dois e três",
        ]);
        for (i, text) in ["um", "dois", "três"].iter().enumerate() {
            fixture
                .transcriptions
                .insert(&segment(
                    &format!("{i}"),
                    (i as i64) * 3_000,
                    &format!("{text} {}", "x".repeat(60)),
                ))
                .unwrap();
        }

        let service = fixture.service.with_limits(SummaryLimits {
            max_context_chars: 80,
            max_chunks: 2,
            ..SummaryLimits::default()
        });
        let result = service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .expect("chunks");

        assert!(result.updated);
        assert!(result.has_more, "ainda falta o terceiro trecho");
        assert_eq!(result.summary.summarized_until_ms, 3_000);
        assert_eq!(fixture.llm.call_count(), 2);

        let rest = service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .expect("resto");
        assert!(!rest.has_more);
        assert_eq!(rest.summary.summarized_until_ms, 6_000);
        assert_eq!(fixture.llm.call_count(), 3);
    }

    #[test]
    fn errors_without_speech_or_session() {
        let empty = fixture(vec!["x"]);
        fixture_error(&empty);

        let no_session = Fixture {
            service: {
                let sessions = Arc::new(InMemorySessionRepository::new());
                SummaryService::new(
                    Arc::new(FakeLlmController::new()),
                    Arc::new(InMemorySettingsStore::new()),
                    sessions,
                    Arc::new(InMemoryTranscriptionsRepository::new()),
                    Arc::new(InMemoryMeetingSummariesRepository::new()),
                    Arc::new(FixedClock(0)),
                )
            },
            llm: Arc::new(FakeLlmController::new()),
            transcriptions: Arc::new(InMemoryTranscriptionsRepository::new()),
            summaries: Arc::new(InMemoryMeetingSummariesRepository::new()),
            sessions: Arc::new(InMemorySessionRepository::new()),
        };
        assert!(no_session
            .service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .is_err());
    }

    fn fixture_error(fixture: &Fixture) {
        let error = fixture
            .service
            .generate(SummaryRequest {
                session_id: None,
                mode: SummaryMode::Auto,
            })
            .unwrap_err()
            .to_string();
        assert!(error.contains("vazia"), "{error}");
    }

    #[test]
    fn build_context_for_tests_starts_at_zero() {
        let segments = vec![segment("1", 0, "começo")];
        let chunks = build_context_for_tests(&segments, None, 1_000);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("começo"));
    }
}
