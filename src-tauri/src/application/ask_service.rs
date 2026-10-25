//! Caso de uso: **perguntar** à IA e registrar no diário da sessão.
//!
//! É a feature "Busca" da aba "Sessão" (BR-053): o usuário pergunta, a IA
//! responde com o **contexto da sessão**, e pergunta + resposta ficam
//! gravadas no diário.
//!
//! Regras que valem aqui:
//!
//! - A pergunta é persistida **antes** da chamada à IA (RNF-06): um crash no
//!   meio deixa a entrada no diário como `pending`.
//! - **Uma pergunta por vez** (single-flight, como as sugestões — BR-042): um
//!   novo envio durante uma resposta em curso não duplica texto.
//! - O escopo de rota é `Capability(Search)`: a Busca pode ter uma fonte de
//!   IA própria (BR-048).
//! - A entrada registra a **origem** (`search`/`preset`/`screenshot`) e o
//!   `provider`/`model` que responderam.
//! - A **memória longa** da sessão (o resumo incremental) entra no prompt como
//!   contexto das partes antigas da reunião; se estiver ausente ou velha, é
//!   atualizada em **background**, em paralelo com a resposta. A falha do
//!   resumo nunca impede a pergunta (prioridade: velocidade da resposta).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;

use uuid::Uuid;

use crate::domain::ai_route::{AiCapability, AiRouteScope};
use crate::domain::journal::{JournalEntry, JournalImage, JournalStatus};
use crate::domain::llm::{CompletionImage, CompletionRequest};
use crate::domain::overview::MeetingSummary;
use crate::domain::transcription::TranscriptSegment;
use crate::error::{GhostWhisperError, Result};
use crate::ports::ask_controller::{AskController, AskOutcome, AskRequest};
use crate::ports::clock::Clock;
use crate::ports::event_publisher::{EventPublisher, JournalPhase, JournalUpdate};
use crate::ports::journal_repository::JournalRepository;
use crate::ports::llm_controller::LlmController;
use crate::ports::repositories::{SessionRepository, TranscriptionsRepository};
use crate::ports::summary_generator::{SummaryGenerator, SummaryMode, SummaryRequest};

use super::answer_summary;
use super::context;
use super::prompts::assistant as assistant_prompts;
use super::session_lookup::resolve_session;

/// Teto do contexto enviado com a pergunta (mesma ordem de grandeza da
/// sugestão — BR-024).
const CONTEXT_MAX_CHARS: usize = 6_000;
/// Janela de contexto (ms) — a mesma das sugestões.
const CONTEXT_WINDOW_MS: i64 = 180_000;
/// Teto de falas no contexto.
const CONTEXT_MAX_SEGMENTS: usize = 40;
/// Quantos resumos anteriores entram no contexto (janela deslizante).
const CONTEXT_SUMMARIES: usize = 5;
/// Quantos resumos de perguntas entram quando o resumo da reunião está no
/// prompt — o resumo da reunião já cobre o contexto longo.
const CONTEXT_SUMMARIES_WITH_MEETING: usize = 3;
/// Teto de caracteres dos resumos no prompt.
const CONTEXT_SUMMARIES_MAX_CHARS: usize = 1_500;
/// Teto de caracteres do resumo da reunião no prompt.
///
/// Um resumo típico fica bem abaixo disso (o teto de saída é 2.500 tokens);
/// quando passa, o excesso é cortado pelo **fim** — a parte recente já está
/// coberta pela transcrição ao vivo.
const CONTEXT_MEETING_SUMMARY_MAX_CHARS: usize = 6_000;
/// Cooldown depois de uma falha do resumo em background (ms).
///
/// Evita martelar o provedor a cada pergunta quando o resumo está quebrado.
const SUMMARY_REFRESH_COOLDOWN_MS: i64 = 60_000;
/// Teto de tokens da resposta.
///
/// Folgado o bastante para uma resposta com código + exemplo **e** o bloco de
/// resumo no fim (modelos que contam raciocínio no mesmo teto também cabem).
const MAX_TOKENS: u32 = 1_500;
/// Temperatura da resposta.
const TEMPERATURE: f32 = 0.3;

/// Prompt usado quando o usuário manda **só o print**, sem texto.
///
/// Paridade com o `system_prompt_image` do Perssua ("Analyze this image"): a
/// IA decide o que fazer com a imagem (BR-053).
pub const DEFAULT_IMAGE_PROMPT: &str =
    "Analise a imagem da tela e responda ao que for mais útil para a conversa.";

/// Regras do **pedido**. A persona vem do agente **Padrão** (BR-059).
///
/// O Padrão identifica a cena (reunião, entrevista técnica, conversa geral) e
/// **nunca devolve a pergunta** — é o que faz uma pergunta ditada, com erro de
/// transcrição, ser respondida em vez de "você quis dizer…?".
const ASK_RULES: &str = "Esta é uma pergunta direta do usuário (não uma sugestão \
     ao vivo): responda em **Markdown completo**, de uma vez. Quando a pergunta \
     pedir código, use um bloco com a linguagem (por exemplo ```python) e inclua \
     um exemplo de uso; quando pedir uma explicação, traga os passos. Seja \
     direto, mas não omita o que a pergunta pede. \
     A pergunta pode ter vindo de ditado (voz) e chegar com erros de \
     transcrição: interprete a intenção e responda; nunca devolva a pergunta nem \
     peça confirmação — se algo estiver ambíguo, assuma a interpretação mais \
     provável e diga em uma linha o que assumiu. \
     A transcrição da reunião é contexto, não requisito: perguntas gerais \
     (código, conceitos, exercícios) são respondidas normalmente, sem exigir que \
     estejam na reunião. Não invente o que foi dito nela. \
     No bloco <transcript>, ME é o usuário (microfone) e THEY é o outro lado \
     (áudio do sistema); ambos são fala de participante, com possíveis erros de \
     reconhecimento e atribuição imperfeita — interprete pelo contexto. \
     Quando houver um bloco <meeting_summary>, ele é o contexto das partes \
     anteriores da reunião; o <transcript> é a fala mais recente e tem \
     precedência em caso de conflito. Não invente o que não estiver no resumo \
     nem na transcrição e não repita o resumo literalmente. \
     Depois da resposta completa, numa linha própria, escreva exatamente: \
     <!--resumo: até 240 caracteres resumindo o que você respondeu--> \
     Esse bloco é apenas o fecho; a resposta acima dele deve estar completa.";

/// Prompt de sistema da Busca: o agente **Padrão** (o que se adapta à cena) +
/// as regras do pedido. Não há persona de reunião fixa: a pergunta pode ser
/// sobre qualquer assunto e pode ter vindo de ditado.
///
/// **Não** entra aqui o `NEVER_ASK_BACK_RULE` dos agentes ao vivo: ele manda
/// parar quando "o contexto está vazio", e aqui o contexto é opcional — a
/// regra derrubaria justamente a pergunta feita sem reunião. O Padrão já diz
/// que o copiloto "nunca devolve a pergunta" e que assume em vez de perguntar.
fn system_prompt() -> String {
    format!(
        "{}\n\n{}",
        assistant_prompts::DEFAULT_SYSTEM_PROMPT,
        ASK_RULES
    )
}

/// Estado do refresh do resumo disparado pelo **Perguntar**.
///
/// - **single-flight**: no máximo uma atualização por sessão; a pergunta
///   seguinte não empilha outra (a aba Resumo tem o próprio `inFlight` no
///   frontend).
/// - **cooldown de falha**: se o resumo falhou, não tenta de novo a cada
///   pergunta (evita martelar o provedor).
#[derive(Default)]
struct SummaryRefreshState {
    inner: Mutex<SummaryRefreshInner>,
}

#[derive(Default)]
struct SummaryRefreshInner {
    in_flight: HashSet<String>,
    last_failure_ms: HashMap<String, i64>,
}

impl SummaryRefreshState {
    /// Reserva a atualização da sessão; `false` = já em voo ou em cooldown.
    fn try_begin(&self, session_id: &str, now_ms: i64) -> bool {
        let Ok(mut inner) = self.inner.lock() else {
            return false;
        };
        if inner.in_flight.contains(session_id) {
            return false;
        }
        if let Some(last) = inner.last_failure_ms.get(session_id) {
            if now_ms.saturating_sub(*last) < SUMMARY_REFRESH_COOLDOWN_MS {
                return false;
            }
        }
        inner.in_flight.insert(session_id.to_string());
        true
    }

    /// Libera a sessão (chamado no fim da thread, inclusive em panic).
    fn finish(&self, session_id: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.in_flight.remove(session_id);
        }
    }

    /// Registra a falha para o cooldown.
    fn note_failure(&self, session_id: &str, now_ms: i64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_failure_ms.insert(session_id.to_string(), now_ms);
        }
    }
}

/// Limpa o "em voo" mesmo se a thread do resumo entrar em panic.
struct SummaryRefreshGuard {
    state: Arc<SummaryRefreshState>,
    session_id: String,
}

impl Drop for SummaryRefreshGuard {
    fn drop(&mut self) {
        self.state.finish(&self.session_id);
    }
}

/// Pergunta à IA e registra no diário da sessão.
pub struct AskService {
    llm: Arc<dyn LlmController>,
    sessions: Arc<dyn SessionRepository>,
    transcriptions: Arc<dyn TranscriptionsRepository>,
    journal: Arc<dyn JournalRepository>,
    /// Resumo da sessão: memória longa do Perguntar (`get`) e atualização
    /// incremental em background (`generate`).
    summaries: Arc<dyn SummaryGenerator>,
    clock: Arc<dyn Clock>,
    publisher: Arc<dyn EventPublisher>,
    /// Serializa as perguntas: uma por vez (BR-042).
    gate: Arc<Mutex<()>>,
    /// Single-flight/cooldown do refresh do resumo em background.
    summary_refresh: Arc<SummaryRefreshState>,
}

impl AskService {
    /// Cria o serviço com as portas resolvidas.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        llm: Arc<dyn LlmController>,
        sessions: Arc<dyn SessionRepository>,
        transcriptions: Arc<dyn TranscriptionsRepository>,
        journal: Arc<dyn JournalRepository>,
        summaries: Arc<dyn SummaryGenerator>,
        clock: Arc<dyn Clock>,
        publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            llm,
            sessions,
            transcriptions,
            journal,
            summaries,
            clock,
            publisher,
            gate: Arc::new(Mutex::new(())),
            summary_refresh: Arc::new(SummaryRefreshState::default()),
        }
    }

    /// Sessão da pergunta: a pedida, a ativa ou uma **nova**.
    ///
    /// A pergunta pode **abrir a sessão** (BR-052): sem sessão nenhuma, o
    /// diário ganha uma sessão própria — é ela que a aba Sessão passa a
    /// mostrar. Com uma sessão ativa (gravação), a pergunta entra nela.
    fn ensure_session(&self, requested: Option<String>) -> Result<crate::domain::session::Session> {
        if let Some(id) = requested {
            if !id.trim().is_empty() {
                return resolve_session(self.sessions.as_ref(), Some(id));
            }
        }

        // Sessão ativa (gravação ou uma sessão de perguntas já aberta).
        let active = self
            .sessions
            .list_recent(20)?
            .into_iter()
            .find(|session| session.status == crate::domain::session::SessionStatus::Active);
        if let Some(session) = active {
            return Ok(session);
        }

        // Nenhuma sessão ativa: encerra eventuais órfãs e abre uma sessão de
        // perguntas (sem captura de áudio).
        let now = self.clock.now_ms();
        let _ = self.sessions.end_all_active(now);
        let session = crate::domain::session::Session {
            id: Uuid::new_v4().to_string(),
            started_at_ms: now,
            ended_at_ms: None,
            provider: "local".to_string(),
            model: None,
            transcription_model: None,
            user_id: None,
            subject: None,
            overview: None,
            status: crate::domain::session::SessionStatus::Active,
        };
        self.sessions.create(&session)?;
        tracing::info!(session_id = session.id, "sessão de perguntas aberta");
        Ok(session)
    }

    /// Milissegundos desde o início da sessão (a escala do diário — BR-019).
    fn elapsed_ms(&self, started_at_ms: &i64) -> i64 {
        (self.clock.now_ms() - *started_at_ms).max(0)
    }

    /// Contexto de uma pergunta: o **delta** da transcrição + os resumos +
    /// a memória longa da reunião.
    ///
    /// O corte é o instante da última pergunta **respondida** (BR-053): o que
    /// veio antes dela já foi usado para responder e voltaria a ser enviado à
    /// toa. A janela de tempo continua como rede de segurança (uma pergunta
    /// depois de uma pausa longa não manda meia hora de fala).
    fn build_context(&self, session_id: &str, now_rel_ms: i64) -> AskContext {
        let entries = self.journal.list_by_session(session_id).unwrap_or_default();

        // O início da última pergunta respondida (o par pergunta/resposta dela
        // já está coberto pelo resumo; a fala durante a resposta é nova).
        let since = entries
            .iter()
            .rev()
            .find(|entry| entry.status == JournalStatus::Answered)
            .map(|entry| entry.asked_at_ms);

        let segments: Vec<TranscriptSegment> = self
            .transcriptions
            .list_by_session(session_id)
            .unwrap_or_default();
        let transcript = match since {
            Some(since) => context::context_since(
                &segments,
                since,
                now_rel_ms,
                CONTEXT_WINDOW_MS,
                CONTEXT_MAX_SEGMENTS,
                CONTEXT_MAX_CHARS,
            ),
            None => context::recent_context_live(
                &segments,
                now_rel_ms,
                CONTEXT_WINDOW_MS,
                CONTEXT_MAX_SEGMENTS,
                CONTEXT_MAX_CHARS,
            ),
        };

        // Memória longa: leitura barata (uma linha) e **não-fatal** — uma falha
        // no banco não pode derrubar a pergunta.
        let meeting_summary = self
            .summaries
            .get(Some(session_id.to_string()))
            .ok()
            .flatten();
        let has_meeting_summary = meeting_summary
            .as_ref()
            .is_some_and(|summary| !summary.markdown.trim().is_empty());
        // Com o resumo da reunião, a janela de perguntas antigas pode ser menor:
        // o resumo já cobre o contexto longo (mantém o prompt no orçamento).
        let summary_limit = if has_meeting_summary {
            CONTEXT_SUMMARIES_WITH_MEETING
        } else {
            CONTEXT_SUMMARIES
        };

        AskContext {
            transcript,
            summaries: recent_summaries(&entries, summary_limit, CONTEXT_SUMMARIES_MAX_CHARS),
            meeting_summary,
            has_final_speech: segments
                .iter()
                .any(|segment| segment.is_final && !segment.text.trim().is_empty()),
        }
    }

    /// Monta o pedido: resumo da reunião + resumos de perguntas + transcrição +
    /// pergunta (+ imagens).
    fn build_request(
        &self,
        question: &str,
        images: &[CompletionImage],
        context: AskContext,
    ) -> CompletionRequest {
        let mut prompt = String::new();
        // Memória longa primeiro (partes antigas); o transcript vem depois e
        // tem precedência no prompt (ver `ASK_RULES`).
        if let Some(summary) = context.meeting_summary.as_ref() {
            let markdown = summary.markdown.trim();
            if !markdown.is_empty() {
                prompt.push_str("<meeting_summary>\n");
                if let Some(subject) = summary
                    .subject
                    .as_deref()
                    .map(str::trim)
                    .filter(|subject| !subject.is_empty())
                {
                    prompt.push_str("SUBJECT: ");
                    prompt.push_str(subject);
                    prompt.push_str("\n\n");
                }
                prompt.push_str(&truncate_chars(markdown, CONTEXT_MEETING_SUMMARY_MAX_CHARS));
                prompt.push_str("\n</meeting_summary>\n\n");
            }
        }
        if !context.summaries.is_empty() {
            prompt.push_str("<previous_questions>\n");
            for summary in &context.summaries {
                prompt.push_str("- ");
                prompt.push_str(summary);
                prompt.push('\n');
            }
            prompt.push_str("</previous_questions>\n\n");
        }
        if let Some(transcript) = context.transcript {
            prompt.push_str("<transcript>\n");
            prompt.push_str(&transcript.text);
            prompt.push_str("\n</transcript>\n\n");
        }
        prompt.push_str(question);

        CompletionRequest::user(prompt)
            .with_system(system_prompt())
            .with_max_tokens(MAX_TOKENS)
            .with_temperature(TEMPERATURE)
            .with_images(images.to_vec())
    }

    /// O resumo precisa de atualização? Ausente sempre precisa; existente só
    /// quando ficou para trás da janela que o transcript recente já cobre.
    fn summary_refresh_needed(summary: Option<&MeetingSummary>, now_rel_ms: i64) -> bool {
        match summary {
            None => true,
            Some(summary) => {
                now_rel_ms.saturating_sub(summary.summarized_until_ms) > CONTEXT_WINDOW_MS
            }
        }
    }

    /// Atualiza o resumo em uma thread própria, **sem** participar da resposta.
    ///
    /// A resposta já foi montada com o snapshot atual e começa logo depois; o
    /// resultado deste trabalho só beneficia a próxima pergunta (ou a aba
    /// Resumo). Falha é log + cooldown, nunca erro da pergunta.
    fn spawn_summary_refresh(&self, session_id: &str) {
        let now_ms = self.clock.now_ms();
        if !self.summary_refresh.try_begin(session_id, now_ms) {
            return;
        }

        let generator = Arc::clone(&self.summaries);
        let clock = Arc::clone(&self.clock);
        let refresh = Arc::clone(&self.summary_refresh);
        let id = session_id.to_string();
        let spawned = thread::Builder::new()
            .name("ask-summary".into())
            .spawn(move || {
                let _guard = SummaryRefreshGuard {
                    state: Arc::clone(&refresh),
                    session_id: id.clone(),
                };
                match generator.generate(SummaryRequest {
                    session_id: Some(id.clone()),
                    mode: SummaryMode::Auto,
                }) {
                    Ok(result) => tracing::debug!(
                        session_id = %id,
                        updated = result.updated,
                        has_more = result.has_more,
                        "resumo atualizado em background para o Perguntar"
                    ),
                    Err(error) => {
                        tracing::warn!(
                            session_id = %id,
                            "falha ao atualizar o resumo em background: {error}"
                        );
                        refresh.note_failure(&id, clock.now_ms());
                    }
                }
            });
        if let Err(error) = spawned {
            // A reserva foi feita antes do spawn; libera para a próxima.
            self.summary_refresh.finish(session_id);
            tracing::warn!("falha ao iniciar thread do resumo em background: {error}");
        }
    }
}

/// Contexto montado para uma pergunta (BR-053).
struct AskContext {
    /// Falas desde a última pergunta respondida (ou a janela recente).
    transcript: Option<context::RecentContext>,
    /// Resumos dos pares pergunta/resposta anteriores (mais antigo primeiro).
    summaries: Vec<String>,
    /// Resumo incremental da reunião (memória longa), quando existe.
    meeting_summary: Option<MeetingSummary>,
    /// Há fala final utilizável — evita gerar resumo para uma sessão criada
    /// só pela própria pergunta (sem transcrição nenhuma).
    has_final_speech: bool,
}

/// Corta um texto no teto **sem quebrar UTF-8**, sinalizando o corte.
fn truncate_chars(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

/// Últimos `limit` resumos da sessão, em ordem cronológica, com teto de chars.
///
/// A janela deslizante mantém o prompt pequeno: os resumos mais antigos saem
/// primeiro; o teto é aplicado do mais novo para o mais antigo.
fn recent_summaries(entries: &[JournalEntry], limit: usize, max_chars: usize) -> Vec<String> {
    if limit == 0 || max_chars == 0 {
        return Vec::new();
    }
    let mut picked: Vec<&JournalEntry> = entries
        .iter()
        .filter(|entry| {
            entry
                .summary
                .as_deref()
                .map(|text| !text.trim().is_empty())
                .unwrap_or(false)
        })
        .collect();
    if picked.len() > limit {
        picked = picked.split_off(picked.len() - limit);
    }

    let mut kept: Vec<String> = Vec::new();
    let mut chars = 0usize;
    for entry in picked.iter().rev() {
        let text = entry
            .summary
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string();
        let len = text.chars().count();
        if !kept.is_empty() && chars + len > max_chars {
            break;
        }
        chars += len;
        kept.push(text);
    }
    kept.reverse();
    kept
}

impl AskController for AskService {
    fn ask(&self, request: AskRequest) -> Result<AskOutcome> {
        let question = request.question.trim().to_string();
        // Print sem texto é permitido: a IA decide o que fazer com a imagem
        // (paridade com o `system_prompt_image` do Perssua).
        if question.is_empty() && request.images.is_empty() {
            return Err(GhostWhisperError::Config(
                "a pergunta está vazia".to_string(),
            ));
        }
        // O diário guarda a pergunta **do usuário** (vazia quando foi só o
        // print): é ela que a UI mostra ("se houver pergunta"). O prompt padrão
        // vale só para o modelo.
        let prompt = if question.is_empty() {
            DEFAULT_IMAGE_PROMPT.to_string()
        } else {
            question.clone()
        };

        let session = self.ensure_session(request.session_id.clone())?;
        let asked_at_ms = self.elapsed_ms(&session.started_at_ms);

        // A pergunta entra no diário antes de qualquer chamada: um crash no
        // meio deixa a entrada `pending` (RNF-06).
        let entry = JournalEntry {
            id: Uuid::new_v4().to_string(),
            session_id: session.id.clone(),
            question: question.clone(),
            answer: None,
            asked_at_ms,
            answered_at_ms: None,
            created_at_ms: 0,
            status: JournalStatus::Pending,
            request_source: request.source,
            provider: None,
            model: None,
            error: None,
            summary: None,
        };
        self.journal.insert_question(&entry)?;

        // As imagens anexadas entram no diário junto da pergunta (BR-056).
        for image in &request.images {
            self.journal.attach_image(&JournalImage {
                id: Uuid::new_v4().to_string(),
                entry_id: entry.id.clone(),
                mime: image.mime.clone(),
                data: image.data.clone(),
                width: None,
                height: None,
            })?;
        }

        self.publisher.emit_journal(&JournalUpdate {
            entry_id: entry.id.clone(),
            phase: JournalPhase::Pending,
            text: question.clone(),
            error: None,
        });

        // Single-flight: uma pergunta por vez (BR-042).
        let _guard = self
            .gate
            .lock()
            .map_err(|_| GhostWhisperError::Other("lock de perguntas envenenado".to_string()))?;

        let context = self.build_context(&session.id, asked_at_ms);
        // O snapshot do resumo vai no prompt abaixo. A atualização (se faltar
        // ou estiver velha) roda em paralelo com a resposta e **nunca** é
        // aguardada: velocidade da pergunta primeiro.
        let refresh_summary = context.has_final_speech
            && Self::summary_refresh_needed(context.meeting_summary.as_ref(), asked_at_ms);
        let completion_request = self.build_request(&prompt, &request.images, context);
        if refresh_summary {
            self.spawn_summary_refresh(&session.id);
        }

        let mut streamed = String::new();
        let result = self.llm.complete_streaming_for(
            &AiRouteScope::Capability(AiCapability::Search),
            &completion_request,
            &mut |text| {
                streamed.push_str(text);
                // O bloco de resumo do fim não aparece no streaming.
                let visible = answer_summary::visible_stream(&streamed).to_string();
                self.publisher.emit_journal(&JournalUpdate {
                    entry_id: entry.id.clone(),
                    phase: JournalPhase::Streaming,
                    text: visible,
                    error: None,
                });
            },
        );

        match result {
            Ok(completion) => {
                let raw = if completion.text.trim().is_empty() {
                    streamed.clone()
                } else {
                    completion.text.clone()
                };
                // A resposta exibida/gravada sai **sem** o trailer; o resumo
                // vai para o diário e alimenta a próxima pergunta. Sem o
                // bloco (modelo desobediente ou resposta cortada), o digest
                // determinístico cobre (BR-053).
                let answer = answer_summary::without_summary(&raw).to_string();
                let summary = answer_summary::extract(&raw).or_else(|| {
                    let digest = answer_summary::digest(&question, &answer);
                    (!digest.is_empty()).then_some(digest)
                });
                let answered_at_ms = self.elapsed_ms(&session.started_at_ms);
                self.journal.complete_answer(
                    &entry.id,
                    &answer,
                    answered_at_ms,
                    completion.provider.clone(),
                    Some(completion.model.clone()),
                    summary.as_deref(),
                )?;
                self.publisher.emit_journal(&JournalUpdate {
                    entry_id: entry.id.clone(),
                    phase: JournalPhase::Answered,
                    text: answer.clone(),
                    error: None,
                });
                Ok(AskOutcome {
                    entry_id: entry.id,
                    answer,
                })
            }
            Err(error) => {
                let message = error.to_string();
                self.journal
                    .fail(&entry.id, JournalStatus::Error, Some(message.clone()))?;
                self.publisher.emit_journal(&JournalUpdate {
                    entry_id: entry.id.clone(),
                    phase: JournalPhase::Error,
                    text: answer_summary::without_summary(&streamed).to_string(),
                    error: Some(message.clone()),
                });
                Err(error)
            }
        }
    }

    fn cancel(&self, entry_id: &str) -> Result<()> {
        self.journal.fail(entry_id, JournalStatus::Cancelled, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::adapters::persistence::memory::in_memory_journal_repository::entry as journal_entry;
    use crate::adapters::persistence::memory::in_memory_journal_repository::InMemoryJournalRepository;
    use crate::adapters::persistence::memory::in_memory_session_repository::InMemorySessionRepository;
    use crate::adapters::persistence::memory::in_memory_transcriptions_repository::InMemoryTranscriptionsRepository;
    use crate::adapters::system::system_clock::MutableClock;
    use crate::domain::journal::JournalRequestSource;
    use crate::domain::llm::Completion;
    use crate::domain::overview::MeetingSummary;
    use crate::domain::session::{Session, SessionStatus};
    use crate::domain::transcription::{Speaker, TranscriptSegment};
    use crate::ports::event_publisher::LlmSourceUsage;
    use crate::ports::session_controller::AudioChannel;
    use crate::ports::suggestion_generator::Suggestion;
    use crate::ports::summary_generator::{SummaryGenerator, SummaryRequest, SummaryResult};

    /// Publicador que guarda os eventos do diário.
    #[derive(Default)]
    struct RecordingPublisher {
        updates: Mutex<Vec<JournalUpdate>>,
    }

    impl EventPublisher for RecordingPublisher {
        fn emit_segment(&self, _segment: &TranscriptSegment) {}
        fn emit_audio_level(
            &self,
            _channel: AudioChannel,
            _level: crate::domain::audio::AudioLevel,
        ) {
        }
        fn emit_session_state(&self, _state: &crate::ports::event_publisher::SessionStateEvent) {}
        fn emit_insight(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion(&self, _suggestion: &Suggestion) {}
        fn emit_suggestion_cadence(
            &self,
            _cadence: &crate::ports::event_publisher::SuggestionCadenceEvent,
        ) {
        }
        fn emit_asr_status(&self, _status: &crate::domain::transcription::AsrStatus) {}
        fn emit_llm_source_used(&self, _usage: &LlmSourceUsage) {}
        fn emit_journal(&self, update: &JournalUpdate) {
            self.updates.lock().expect("lock").push(update.clone());
        }
    }

    /// LLM que devolve uma resposta fixa (ou falha) e guarda o pedido.
    struct FakeLlm {
        response: String,
        fails: bool,
        calls: AtomicUsize,
        request: Mutex<Option<CompletionRequest>>,
    }

    impl FakeLlm {
        fn ok(response: &str) -> Self {
            Self {
                response: response.to_string(),
                fails: false,
                calls: AtomicUsize::new(0),
                request: Mutex::new(None),
            }
        }

        fn failing() -> Self {
            Self {
                response: String::new(),
                fails: true,
                calls: AtomicUsize::new(0),
                request: Mutex::new(None),
            }
        }
    }

    impl LlmController for FakeLlm {
        fn provider_label(&self) -> Result<String> {
            Ok("fake:test".to_string())
        }

        fn validate(&self) -> Result<String> {
            Ok("ok".to_string())
        }

        fn complete(&self, request: &CompletionRequest) -> Result<Completion> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.request.lock().expect("lock") = Some(request.clone());
            if self.fails {
                return Err(GhostWhisperError::Other("provider fora".to_string()));
            }
            Ok(Completion {
                text: self.response.clone(),
                model: "fake-model".to_string(),
                provider: Some("fake".to_string()),
                usage: None,
            })
        }

        fn complete_streaming(
            &self,
            request: &CompletionRequest,
            on_text: &mut dyn FnMut(&str),
        ) -> Result<Completion> {
            let completion = self.complete(request)?;
            on_text(&completion.text);
            Ok(completion)
        }
    }

    /// Cancela a geração do resumo até `release` (prova de paralelismo).
    struct SummaryGate {
        started: std::sync::mpsc::Sender<()>,
        release: std::sync::mpsc::Receiver<()>,
    }

    /// Resumo fake: `get` devolve o que estiver guardado; `generate` pode
    /// falhar ou bloquear num gate.
    #[derive(Default)]
    struct FakeSummary {
        stored: Mutex<Option<MeetingSummary>>,
        fail: bool,
        calls: AtomicUsize,
        gate: Mutex<Option<SummaryGate>>,
    }

    impl FakeSummary {
        fn empty() -> Self {
            Self::default()
        }

        fn with_markdown(markdown: &str, summarized_until_ms: i64) -> Self {
            Self {
                stored: Mutex::new(Some(MeetingSummary {
                    session_id: "s1".to_string(),
                    subject: Some("Reunião de teste".to_string()),
                    markdown: markdown.to_string(),
                    summarized_from_ms: 0,
                    summarized_until_ms,
                    updated_at_ms: 1,
                })),
                ..Self::default()
            }
        }

        fn failing() -> Self {
            Self {
                fail: true,
                ..Self::default()
            }
        }

        fn gated(gate: SummaryGate) -> Self {
            Self {
                gate: Mutex::new(Some(gate)),
                ..Self::default()
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl SummaryGenerator for FakeSummary {
        fn get(&self, _session_id: Option<String>) -> Result<Option<MeetingSummary>> {
            Ok(self.stored.lock().expect("lock").clone())
        }

        fn generate(&self, request: SummaryRequest) -> Result<SummaryResult> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(gate) = self.gate.lock().expect("lock").take() {
                let _ = gate.started.send(());
                let _ = gate.release.recv_timeout(std::time::Duration::from_secs(2));
            }
            if self.fail {
                return Err(GhostWhisperError::Other("resumo fora".to_string()));
            }
            let summary = self
                .stored
                .lock()
                .expect("lock")
                .clone()
                .unwrap_or_else(|| MeetingSummary {
                    session_id: request.session_id.unwrap_or_default(),
                    subject: None,
                    markdown: String::new(),
                    summarized_from_ms: 0,
                    summarized_until_ms: 0,
                    updated_at_ms: 0,
                });
            Ok(SummaryResult {
                summary,
                updated: false,
                has_more: false,
            })
        }
    }

    struct Fixture {
        service: AskService,
        llm: Arc<FakeLlm>,
        journal: Arc<InMemoryJournalRepository>,
        sessions: Arc<InMemorySessionRepository>,
        transcriptions: Arc<InMemoryTranscriptionsRepository>,
        publisher: Arc<RecordingPublisher>,
        clock: Arc<MutableClock>,
    }

    fn fixture_with_summary(llm: Arc<FakeLlm>, summary: Arc<FakeSummary>) -> Fixture {
        let sessions = Arc::new(InMemorySessionRepository::new());
        let transcriptions = Arc::new(InMemoryTranscriptionsRepository::new());
        let journal = Arc::new(InMemoryJournalRepository::new());
        let publisher = Arc::new(RecordingPublisher::default());
        let clock = Arc::new(MutableClock::new(5_000));
        let service = AskService::new(
            llm.clone(),
            sessions.clone(),
            transcriptions.clone(),
            journal.clone(),
            summary.clone(),
            clock.clone(),
            publisher.clone(),
        );
        Fixture {
            service,
            llm,
            journal,
            sessions,
            transcriptions,
            publisher,
            clock,
        }
    }

    fn fixture(llm: Arc<FakeLlm>) -> Fixture {
        fixture_with_summary(llm, Arc::new(FakeSummary::empty()))
    }

    /// Fala final na sessão `s1` (tempos relativos ao início dela).
    fn speech(id: &str, at: i64, text: &str) -> TranscriptSegment {
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

    /// Entrada respondida (com resumo) direto no diário.
    fn answered_entry(f: &Fixture, id: &str, at: i64, summary: &str) {
        f.journal
            .insert_question(&journal_entry(id, "s1", "pergunta antiga", at))
            .expect("entrada");
        f.journal
            .complete_answer(id, "resposta antiga", at + 100, None, None, Some(summary))
            .expect("responde");
    }

    /// Cria a sessão `s1` (início em 1 000; o relógio fixo está em 5 000).
    fn session(fixture: &Fixture) {
        fixture
            .sessions
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
            .expect("cria sessão");
    }

    fn ask(question: &str) -> AskRequest {
        AskRequest {
            session_id: Some("s1".to_string()),
            question: question.to_string(),
            source: JournalRequestSource::Search,
            images: Vec::new(),
        }
    }

    #[test]
    fn the_ask_uses_the_default_agent_and_never_asks_back() {
        // A pergunta pode vir de ditado (com erros): o agente **Padrão** é o
        // que se adapta à cena e responde — devolver a pergunta é defeito.
        let prompt = system_prompt();
        assert!(prompt.contains(assistant_prompts::DEFAULT_SYSTEM_PROMPT));
        assert!(
            !prompt.contains(assistant_prompts::NEVER_ASK_BACK_RULE),
            "a regra ao vivo manda parar sem contexto — aqui o contexto é opcional"
        );
        assert!(prompt.contains("nunca devolva a pergunta"));
        assert!(prompt.contains("erros de transcrição"));
        assert!(prompt.contains("sem exigir que estejam na reunião"));
        // A persona fixa de reunião saiu.
        assert!(!prompt.contains("perguntas sobre uma reunião em andamento"));
    }

    #[test]
    fn the_ask_explains_the_transcript_tags_without_the_live_priority_rule() {
        // BR-082: a Busca explica ME/THEY, mas **não** herda o contrato ao vivo
        // inteiro — lá a pergunta do usuário é a prioridade, não a fala do THEY.
        let prompt = system_prompt();
        assert!(prompt.contains("ME é o usuário (microfone)"));
        assert!(prompt.contains("THEY é o outro lado"));
        assert!(prompt.contains("atribuição imperfeita"));
        assert!(!prompt.contains("## Reading the transcript"));
        assert!(!prompt.contains("highest-priority context"));
    }

    #[test]
    fn the_system_prompt_asks_for_a_complete_answer_before_the_summary() {
        // Regressão: "não repetir o que já foi dito" + "termine sempre com o
        // resumo" deixava as respostas telegráficas (sem código nem exemplos).
        let prompt = system_prompt();
        assert!(prompt.contains("Markdown completo"));
        assert!(prompt.contains("exemplo de uso"));
        assert!(prompt.contains("<!--resumo:"));
        assert!(!prompt.contains("direta e curta"));
        assert!(!prompt.contains("não repetir"));
    }

    #[test]
    fn the_request_has_room_for_code_and_the_summary() {
        let f = fixture(Arc::new(FakeLlm::ok("x")));
        session(&f);
        f.service
            .ask(ask("inverta uma árvore binária em python"))
            .expect("pergunta");
        let sent = f.llm.request.lock().expect("lock").clone().expect("pedido");
        assert_eq!(sent.max_tokens, Some(MAX_TOKENS));
    }

    #[test]
    fn persists_the_question_and_the_answer() {
        let f = fixture(Arc::new(FakeLlm::ok("a resposta")));
        session(&f);

        let outcome = f.service.ask(ask("o que eu disse?")).expect("pergunta");

        let entries = f.journal.list_by_session("s1").expect("diário");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].question, "o que eu disse?");
        assert_eq!(entries[0].answer.as_deref(), Some("a resposta"));
        assert_eq!(entries[0].status, JournalStatus::Answered);
        assert_eq!(entries[0].model.as_deref(), Some("fake-model"));
        assert_eq!(entries[0].provider.as_deref(), Some("fake"));
        assert_eq!(outcome.answer, "a resposta");
    }

    #[test]
    fn the_instant_is_relative_to_the_session_start() {
        let f = fixture(Arc::new(FakeLlm::ok("a resposta")));
        session(&f);
        f.service.ask(ask("pergunta")).expect("pergunta");

        let entries = f.journal.list_by_session("s1").expect("diário");
        // Relógio fixo em 5 000, sessão começou em 1 000 ⇒ 4 000 ms.
        assert_eq!(entries[0].asked_at_ms, 4_000);
    }

    #[test]
    fn the_question_is_persisted_before_the_answer() {
        let f = fixture(Arc::new(FakeLlm::failing()));
        session(&f);

        let error = f.service.ask(ask("pergunta")).expect_err("falha");
        assert!(error.to_string().contains("provider fora"));

        // A entrada continua no diário, marcada como erro (RNF-06).
        let entries = f.journal.list_by_session("s1").expect("diário");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, JournalStatus::Error);
        assert!(entries[0].answer.is_none());
    }

    #[test]
    fn publishes_pending_streaming_and_answered() {
        let f = fixture(Arc::new(FakeLlm::ok("a resposta")));
        session(&f);

        f.service.ask(ask("pergunta")).expect("pergunta");

        let updates = f.publisher.updates.lock().expect("lock").clone();
        let phases: Vec<JournalPhase> = updates.iter().map(|update| update.phase).collect();
        assert_eq!(phases[0], JournalPhase::Pending);
        assert!(phases.contains(&JournalPhase::Streaming));
        assert_eq!(phases[phases.len() - 1], JournalPhase::Answered);
    }

    #[test]
    fn an_empty_question_without_an_image_is_refused() {
        let f = fixture(Arc::new(FakeLlm::ok("resposta")));
        session(&f);
        assert!(f.service.ask(ask("   ")).is_err());
        assert!(f.journal.list_by_session("s1").expect("diário").is_empty());
    }

    #[test]
    fn an_image_without_a_question_uses_the_default_prompt() {
        // Print sem texto: a IA decide o que fazer com a imagem (BR-053).
        let f = fixture(Arc::new(FakeLlm::ok("vejo um código")));
        session(&f);
        let mut request = ask("   ");
        request.images = vec![CompletionImage {
            mime: "image/jpeg".to_string(),
            data: vec![1, 2, 3],
        }];
        request.source = JournalRequestSource::Screenshot;

        f.service.ask(request).expect("pergunta com print");

        let entries = f.journal.list_by_session("s1").expect("diário");
        assert_eq!(entries.len(), 1);
        // O diário guarda a pergunta **do usuário** (vazia: foi só o print)...
        assert!(entries[0].question.is_empty());
        assert_eq!(entries[0].request_source, JournalRequestSource::Screenshot);
        assert_eq!(entries[0].status, JournalStatus::Answered);
        assert_eq!(
            f.journal.images_of(&entries[0].id).expect("imagens").len(),
            1
        );
        // ...e o modelo recebe o prompt padrão da imagem.
        let sent = f
            .llm
            .request
            .lock()
            .expect("lock")
            .clone()
            .expect("pedido enviado");
        assert!(
            sent.prompt.contains(DEFAULT_IMAGE_PROMPT),
            "{}",
            sent.prompt
        );
    }

    #[test]
    fn the_answer_drops_the_summary_trailer_and_stores_it() {
        let f = fixture(Arc::new(FakeLlm::ok(
            "Use eventos.\n<!--resumo: eventos com idempotência-->",
        )));
        session(&f);

        let outcome = f.service.ask(ask("como?")).expect("pergunta");

        assert_eq!(outcome.answer, "Use eventos.");
        let entries = f.journal.list_by_session("s1").expect("diário");
        assert_eq!(entries[0].answer.as_deref(), Some("Use eventos."));
        assert_eq!(
            entries[0].summary.as_deref(),
            Some("eventos com idempotência")
        );
    }

    #[test]
    fn without_the_trailer_the_digest_is_stored() {
        let f = fixture(Arc::new(FakeLlm::ok("a resposta")));
        session(&f);

        f.service.ask(ask("o que eu disse?")).expect("pergunta");

        let entries = f.journal.list_by_session("s1").expect("diário");
        assert_eq!(
            entries[0].summary.as_deref(),
            Some("P: o que eu disse? / R: a resposta")
        );
    }

    #[test]
    fn a_new_question_sends_only_the_delta_and_the_previous_summary() {
        let f = fixture(Arc::new(FakeLlm::ok("primeira resposta")));
        session(&f);
        // Fala que a primeira pergunta já usou.
        f.transcriptions
            .insert(&speech("t1", 2_000, "fala antiga"))
            .expect("fala");

        f.service.ask(ask("primeira?")).expect("pergunta");
        let first = f.llm.request.lock().expect("lock").clone().expect("pedido");
        assert!(first.prompt.contains("fala antiga"), "{}", first.prompt);
        assert!(
            !first.prompt.contains("<previous_questions>"),
            "a primeira não tem resumo anterior"
        );

        // Um minuto depois, com uma fala nova.
        f.clock.advance(60_000);
        f.transcriptions
            .insert(&speech("t2", 10_000, "fala nova"))
            .expect("fala");
        f.service.ask(ask("segunda?")).expect("pergunta");

        let second = f.llm.request.lock().expect("lock").clone().expect("pedido");
        assert!(
            second.prompt.contains("<previous_questions>"),
            "{}",
            second.prompt
        );
        assert!(
            second
                .prompt
                .contains("P: primeira? / R: primeira resposta"),
            "{}",
            second.prompt
        );
        assert!(second.prompt.contains("fala nova"), "{}", second.prompt);
        assert!(
            !second.prompt.contains("fala antiga"),
            "o que já foi respondido não volta: {}",
            second.prompt
        );
    }

    #[test]
    fn the_cut_ignores_pending_and_failed_entries() {
        let f = fixture(Arc::new(FakeLlm::ok("x")));
        session(&f);
        answered_entry(&f, "e1", 1_000, "resumo da primeira");

        // Uma pergunta que falhou depois **não** move o corte.
        f.journal
            .insert_question(&journal_entry("e2", "s1", "falhou", 3_000))
            .expect("entrada");
        f.journal
            .fail("e2", JournalStatus::Error, Some("fora".into()))
            .expect("falha");

        f.transcriptions
            .insert(&speech("t1", 2_000, "fala entre as duas"))
            .expect("fala");

        let context = f.service.build_context("s1", 4_000);
        assert_eq!(context.summaries.len(), 1);
        let transcript = context.transcript.expect("transcrição");
        assert!(
            transcript.text.contains("fala entre as duas"),
            "o corte continua na última **respondida**"
        );
    }

    #[test]
    fn the_summaries_are_a_rolling_window_of_five() {
        let f = fixture(Arc::new(FakeLlm::ok("x")));
        session(&f);
        for index in 0..7 {
            answered_entry(
                &f,
                &format!("e{index}"),
                index * 1_000,
                &format!("resumo {index}"),
            );
        }

        let context = f.service.build_context("s1", 10_000);

        assert_eq!(context.summaries.len(), CONTEXT_SUMMARIES);
        assert_eq!(
            context.summaries.first().map(String::as_str),
            Some("resumo 2")
        );
        assert_eq!(
            context.summaries.last().map(String::as_str),
            Some("resumo 6")
        );
    }

    #[test]
    fn the_meeting_summary_goes_into_the_request_before_the_transcript() {
        // A memória longa da reunião entra no prompt, e o transcript recente
        // vem depois (precedência declarada no system prompt).
        let summary = Arc::new(FakeSummary::with_markdown(
            "## Decisões\n\n- Usar Postgres no ledger.",
            4_000,
        ));
        let f = fixture_with_summary(Arc::new(FakeLlm::ok("resposta")), summary);
        session(&f);
        f.transcriptions
            .insert(&speech("t1", 2_000, "fala recente"))
            .expect("fala");

        f.service
            .ask(ask("o que decidimos no começo?"))
            .expect("pergunta");

        let sent = f.llm.request.lock().expect("lock").clone().expect("pedido");
        assert!(sent.prompt.contains("<meeting_summary>"), "{}", sent.prompt);
        assert!(
            sent.prompt.contains("SUBJECT: Reunião de teste"),
            "{}",
            sent.prompt
        );
        assert!(
            sent.prompt.contains("- Usar Postgres no ledger."),
            "{}",
            sent.prompt
        );
        let summary_at = sent.prompt.find("<meeting_summary>").expect("bloco");
        let transcript_at = sent.prompt.find("<transcript>").expect("transcript");
        assert!(
            summary_at < transcript_at,
            "o resumo deve vir antes da fala recente"
        );
        assert!(
            sent.system
                .as_deref()
                .unwrap_or_default()
                .contains("precedência"),
            "o prompt deve declarar a precedência do transcript"
        );
    }

    #[test]
    fn without_a_summary_the_prompt_has_no_meeting_block() {
        let f = fixture(Arc::new(FakeLlm::ok("resposta")));
        session(&f);
        f.transcriptions
            .insert(&speech("t1", 2_000, "fala"))
            .expect("fala");

        f.service.ask(ask("pergunta")).expect("pergunta");

        let sent = f.llm.request.lock().expect("lock").clone().expect("pedido");
        assert!(
            !sent.prompt.contains("<meeting_summary>"),
            "{}",
            sent.prompt
        );
    }

    #[test]
    fn the_meeting_summary_shrinks_the_previous_question_window() {
        // O resumo da reunião já cobre o contexto longo: a janela de perguntas
        // antigas cai de 5 para 3 (orçamento do prompt).
        let summary = Arc::new(FakeSummary::with_markdown("resumo da reunião", 4_000));
        let f = fixture_with_summary(Arc::new(FakeLlm::ok("x")), summary);
        session(&f);
        for index in 0..7 {
            answered_entry(
                &f,
                &format!("e{index}"),
                index * 1_000,
                &format!("resumo {index}"),
            );
        }

        let context = f.service.build_context("s1", 4_000);

        assert!(context.meeting_summary.is_some());
        assert_eq!(context.summaries.len(), CONTEXT_SUMMARIES_WITH_MEETING);
    }

    #[test]
    fn a_fresh_summary_does_not_trigger_a_background_refresh() {
        let summary = Arc::new(FakeSummary::with_markdown("resumo", 4_000));
        let f = fixture_with_summary(Arc::new(FakeLlm::ok("x")), Arc::clone(&summary));
        session(&f);
        f.transcriptions
            .insert(&speech("t1", 2_000, "fala"))
            .expect("fala");

        f.service.ask(ask("pergunta")).expect("pergunta");

        // Nada é disparado quando o cursor já cobre a janela recente.
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(summary.calls(), 0);
    }

    #[test]
    fn a_summary_failure_does_not_block_or_fail_the_answer() {
        let summary = Arc::new(FakeSummary::failing());
        let f = fixture_with_summary(Arc::new(FakeLlm::ok("a resposta")), Arc::clone(&summary));
        session(&f);
        f.transcriptions
            .insert(&speech("t1", 2_000, "fala"))
            .expect("fala");

        let outcome = f.service.ask(ask("pergunta")).expect("pergunta");

        assert_eq!(outcome.answer, "a resposta");
        // O refresh foi tentado, mas o erro ficou no background.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while summary.calls() == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(summary.calls() >= 1, "o resumo deveria ser tentado");
    }

    #[test]
    fn the_summary_refresh_runs_in_parallel_with_the_answer() {
        // O resumo fica bloqueado; a resposta tem de chegar mesmo assim.
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let summary = Arc::new(FakeSummary::gated(SummaryGate {
            started: started_tx,
            release: release_rx,
        }));
        let f = fixture_with_summary(Arc::new(FakeLlm::ok("a resposta")), summary);
        session(&f);
        f.transcriptions
            .insert(&speech("t1", 2_000, "fala"))
            .expect("fala");

        let service = &f.service;
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            scope.spawn(move || {
                let result = service.ask(ask("pergunta"));
                let _ = done_tx.send(result.is_ok());
            });

            started_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .expect("o resumo deveria iniciar em paralelo");
            let answered = done_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .expect("a resposta não pode esperar o resumo");
            assert!(answered);
            release_tx.send(()).expect("libera o resumo");
        });
    }

    #[test]
    fn the_summary_refresh_is_single_flight_and_cools_down() {
        let state = SummaryRefreshState::default();

        assert!(state.try_begin("s1", 1_000));
        assert!(!state.try_begin("s1", 1_000), "já está em voo");
        state.finish("s1");
        assert!(state.try_begin("s1", 2_000), "liberou para a próxima");

        state.finish("s1");
        state.note_failure("s1", 3_000);
        assert!(
            !state.try_begin("s1", 3_000 + SUMMARY_REFRESH_COOLDOWN_MS - 1),
            "durante o cooldown não tenta de novo"
        );
        assert!(
            state.try_begin("s1", 3_000 + SUMMARY_REFRESH_COOLDOWN_MS),
            "depois do cooldown volta a tentar"
        );
    }

    #[test]
    fn the_summary_refresh_needs_new_content() {
        assert!(AskService::summary_refresh_needed(None, 0));
        let fresh = MeetingSummary {
            session_id: "s1".to_string(),
            subject: None,
            markdown: "resumo".to_string(),
            summarized_from_ms: 0,
            summarized_until_ms: 4_000,
            updated_at_ms: 0,
        };
        assert!(!AskService::summary_refresh_needed(Some(&fresh), 4_000));
        assert!(!AskService::summary_refresh_needed(
            Some(&fresh),
            4_000 + CONTEXT_WINDOW_MS
        ));
        assert!(AskService::summary_refresh_needed(
            Some(&fresh),
            4_000 + CONTEXT_WINDOW_MS + 1
        ));
    }

    #[test]
    fn truncate_chars_respects_the_utf8_and_the_cap() {
        assert_eq!(truncate_chars("abc", 10), "abc");
        let cut = truncate_chars("abcdef", 4);
        assert_eq!(cut.chars().count(), 4);
        assert!(cut.ends_with('…'));
        assert!(truncate_chars("qualquer", 0).is_empty());
    }

    #[test]
    fn the_ask_explains_the_meeting_summary_precedence() {
        let prompt = system_prompt();
        assert!(prompt.contains("<meeting_summary>"));
        assert!(prompt.contains("precedência"));
    }

    #[test]
    fn a_question_without_a_session_opens_one() {
        // A pergunta **abre a sessão** quando não há nenhuma (BR-052).
        let f = fixture(Arc::new(FakeLlm::ok("resposta")));
        assert!(f.sessions.list_recent(10).expect("sessões").is_empty());

        let outcome = f
            .service
            .ask(AskRequest {
                session_id: None,
                question: "primeira pergunta".to_string(),
                source: JournalRequestSource::Search,
                images: Vec::new(),
            })
            .expect("pergunta");

        let sessions = f.sessions.list_recent(10).expect("sessões");
        assert_eq!(sessions.len(), 1, "uma sessão foi aberta");
        let opened = &sessions[0];
        assert_eq!(opened.status, SessionStatus::Active);
        let entries = f.journal.list_by_session(&opened.id).expect("diário");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, outcome.entry_id);
    }

    #[test]
    fn the_second_question_reuses_the_opened_session() {
        let f = fixture(Arc::new(FakeLlm::ok("resposta")));
        for question in ["uma", "outra"] {
            f.service
                .ask(AskRequest {
                    session_id: None,
                    question: question.to_string(),
                    source: JournalRequestSource::Search,
                    images: Vec::new(),
                })
                .expect("pergunta");
        }
        let sessions = f.sessions.list_recent(10).expect("sessões");
        assert_eq!(sessions.len(), 1, "o diário acumula na mesma sessão");
        assert_eq!(
            f.journal
                .list_by_session(&sessions[0].id)
                .expect("diário")
                .len(),
            2
        );
    }

    #[test]
    fn without_a_session_the_question_is_refused() {
        let f = fixture(Arc::new(FakeLlm::ok("resposta")));
        let error = f
            .service
            .ask(AskRequest {
                session_id: Some("inexistente".to_string()),
                question: "oi".to_string(),
                source: JournalRequestSource::Search,
                images: Vec::new(),
            })
            .expect_err("sem sessão");
        assert!(error.to_string().contains("não encontrada"));
    }

    #[test]
    fn the_transcript_goes_into_the_request() {
        let f = fixture(Arc::new(FakeLlm::ok("resposta")));
        session(&f);
        f.transcriptions
            .insert(&TranscriptSegment {
                id: "t1".to_string(),
                session_id: "s1".to_string(),
                speaker: Speaker::They,
                text: "o cliente pediu a integração".to_string(),
                start_ms: 2_000,
                end_ms: 2_500,
                is_final: true,
            })
            .expect("insere");

        f.service.ask(ask("o que ele pediu?")).expect("pergunta");

        let request = f.llm.request.lock().expect("lock").clone().expect("pedido");
        assert!(request.prompt.contains("o cliente pediu a integração"));
        assert!(request.prompt.contains("o que ele pediu?"));
        assert!(request.system.is_some());
    }

    #[test]
    fn images_travel_with_the_request() {
        let f = fixture(Arc::new(FakeLlm::ok("resposta")));
        session(&f);
        let mut request = ask("o que está na tela?");
        request.images = vec![CompletionImage {
            mime: "image/jpeg".to_string(),
            data: vec![1, 2, 3],
        }];

        f.service.ask(request).expect("pergunta");

        let captured = f.llm.request.lock().expect("lock").clone().expect("pedido");
        assert_eq!(captured.images.len(), 1);
        assert_eq!(captured.images[0].mime, "image/jpeg");
    }

    #[test]
    fn the_origin_is_recorded() {
        let f = fixture(Arc::new(FakeLlm::ok("resposta")));
        session(&f);
        let mut request = ask("pergunta");
        request.source = JournalRequestSource::Preset;
        f.service.ask(request).expect("pergunta");

        let entries = f.journal.list_by_session("s1").expect("diário");
        assert_eq!(entries[0].request_source, JournalRequestSource::Preset);
    }

    #[test]
    fn cancelling_marks_the_entry() {
        let f = fixture(Arc::new(FakeLlm::ok("resposta")));
        session(&f);
        let outcome = f.service.ask(ask("pergunta")).expect("pergunta");

        f.service.cancel(&outcome.entry_id).expect("cancel");

        let entries = f.journal.list_by_session("s1").expect("diário");
        assert_eq!(entries[0].status, JournalStatus::Cancelled);
    }
}
