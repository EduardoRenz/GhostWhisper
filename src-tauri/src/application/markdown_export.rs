//! Exportação de uma sessão em **Markdown** (task 18).
//!
//! Diferencial em relação ao Perssua (que só oferece "copiar resumo/chat/
//! transcrição"): a sessão inteira vira um documento com a **linha temporal**,
//! incluindo as **sugestões da IA com o tempo em que foram pedidas**.
//!
//! Função pura (`render_markdown` recebe o `now_ms` do chamador) para o
//! resultado ser determinístico e testável.

use crate::domain::session::Session;

use super::session_timeline::{TimelineEntry, TimelineKind};

/** Título usado quando a sessão não foi renomeada. */
pub const UNTITLED: &str = "Sessão sem título";

/// Formata um instante (ms desde a epoch) na hora local do usuário.
pub fn format_local_datetime(ms: i64) -> String {
    use chrono::{Local, TimeZone};

    match Local.timestamp_millis_opt(ms).single() {
        Some(datetime) => datetime.format("%d/%m/%Y %H:%M:%S").to_string(),
        None => ms.to_string(),
    }
}

/// Formata `ms` como `mm:ss` (ou `h:mm:ss`), igual à linha temporal da UI.
pub fn format_clock(ms: i64) -> String {
    let total = (ms.max(0) / 1000) as u64;
    let (hours, minutes, seconds) = (total / 3600, (total % 3600) / 60, total % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// Renderiza a sessão inteira como Markdown.
///
/// `now_ms` é usado apenas para a duração de uma sessão ainda em andamento — a
/// função não lê o relógio para continuar pura.
pub fn render_markdown(session: &Session, entries: &[TimelineEntry], now_ms: i64) -> String {
    let title = session
        .subject
        .as_deref()
        .map(str::trim)
        .filter(|subject| !subject.is_empty())
        .unwrap_or(UNTITLED);

    let end_ms = session.ended_at_ms.unwrap_or(now_ms);
    let duration_ms = (end_ms - session.started_at_ms).max(0);
    let suggestions = entries.iter().filter(|entry| entry.is_suggestion()).count();
    let speeches = entries.len() - suggestions;

    let mut out = String::new();
    out.push_str(&format!("# {title}\n\n"));
    out.push_str(&format!(
        "- **Sessão:** `{}`\n",
        &session.id[..session.id.len().min(8)]
    ));
    out.push_str(&format!(
        "- **Início:** {}\n",
        format_local_datetime(session.started_at_ms)
    ));
    out.push_str(&format!(
        "- **Fim:** {}\n",
        match session.ended_at_ms {
            Some(ended_at) => format_local_datetime(ended_at),
            None => "em andamento".to_string(),
        }
    ));
    out.push_str(&format!("- **Duração:** {}\n", format_clock(duration_ms)));
    out.push_str(&format!(
        "- **Linha temporal:** {speeches} fala(s) · {suggestions} sugestão(ões)\n"
    ));
    out.push_str("\n---\n\n");

    // Resumo vivo (BR-023): entra antes da linha temporal, quando existe.
    if let Some(overview) = session
        .overview
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        out.push_str("## Resumo\n\n");
        out.push_str(overview);
        out.push_str("\n\n---\n\n");
    }

    if entries.is_empty() {
        out.push_str("_Sem falas ou sugestões._\n");
        return out;
    }

    for entry in entries {
        match &entry.kind {
            TimelineKind::Speech { .. } => {
                let speaker = entry.speaker_label().unwrap_or("Fala");
                // Uma fala pode ter quebras de linha: continua a lista alinhada.
                out.push_str(&format!(
                    "- **[{}] {speaker}:** {}\n",
                    format_clock(entry.at_ms),
                    indent_continuation(entry.text(), 2)
                ));
            }
            TimelineKind::Suggestion { text } => {
                out.push('\n');
                out.push_str(&format!(
                    "> 🤖 **Sugestão [{}]** — {}\n\n",
                    format_clock(entry.at_ms),
                    quote_continuation(text)
                ));
            }
        }
    }

    out
}

/// Indenta linhas extras de um texto para não quebrar a lista.
fn indent_continuation(text: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    text.lines()
        .collect::<Vec<_>>()
        .join(&format!("\n{indent}"))
}

/// Prefixa cada linha com `> ` para manter a citação coerente.
fn quote_continuation(text: &str) -> String {
    text.lines().collect::<Vec<_>>().join("\n> ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::MessageRole;
    use crate::domain::session::SessionStatus;
    use crate::domain::transcription::{Speaker, TranscriptSegment};

    fn session(subject: Option<&str>, ended_at_ms: Option<i64>) -> Session {
        Session {
            id: "12345678-abcd".to_string(),
            started_at_ms: 1_700_000_000_000,
            ended_at_ms,
            provider: "local".to_string(),
            model: None,
            transcription_model: Some("tiny".to_string()),
            user_id: None,
            subject: subject.map(str::to_string),
            overview: None,
            status: if ended_at_ms.is_some() {
                SessionStatus::Ended
            } else {
                SessionStatus::Active
            },
        }
    }

    fn speech(at: i64, speaker: Speaker, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            id: format!("seg-{at}"),
            session_id: "12345678-abcd".to_string(),
            speaker,
            text: text.to_string(),
            start_ms: at,
            end_ms: at + 500,
            is_final: true,
        }
    }

    fn suggestion(at: i64, text: &str) -> crate::domain::message::ConversationMessage {
        crate::domain::message::ConversationMessage {
            id: format!("msg-{at}"),
            session_id: "12345678-abcd".to_string(),
            role: MessageRole::Assistant,
            content: text.to_string(),
            timestamp_ms: at,
        }
    }

    fn timeline(
        segments: &[TranscriptSegment],
        messages: &[crate::domain::message::ConversationMessage],
    ) -> Vec<TimelineEntry> {
        crate::application::session_timeline::build_timeline(segments, messages)
    }

    #[test]
    fn formats_clock_like_the_ui() {
        assert_eq!(format_clock(0), "00:00");
        assert_eq!(format_clock(65_000), "01:05");
        assert_eq!(format_clock(3_725_000), "1:02:05");
        assert_eq!(format_clock(-10), "00:00");
    }

    #[test]
    fn renders_header_timeline_and_suggestion_block() {
        let entries = timeline(
            &[
                speech(0, Speaker::They, "podemos fechar assim?"),
                speech(30_000, Speaker::Me, "sim, vou enviar hoje"),
            ],
            &[suggestion(10_000, "Você pode confirmar o prazo.")],
        );

        let markdown = render_markdown(
            &session(Some("Reunião de sexta"), Some(1_700_000_060_000)),
            &entries,
            1_700_000_100_000,
        );

        assert!(markdown.starts_with("# Reunião de sexta\n"));
        assert!(markdown.contains("`12345678`"));
        assert!(markdown.contains("- **Duração:** 01:00"));
        assert!(markdown.contains("2 fala(s) · 1 sugestão(ões)"));
        // A linha temporal traz o tempo em cada item.
        assert!(markdown.contains("- **[00:00] Interlocutor:** podemos fechar assim?"));
        assert!(markdown.contains("- **[00:30] Você:** sim, vou enviar hoje"));
        // A sugestão é um bloco especial, com o tempo em que foi pedida.
        assert!(markdown.contains("> 🤖 **Sugestão [00:10]** — Você pode confirmar o prazo."));
    }

    #[test]
    fn renders_the_summary_before_the_timeline() {
        let mut session = session(Some("Sprint"), Some(1_700_000_060_000));
        session.overview = Some("## Visão geral\n\nDecidimos priorizar o login.".to_string());
        let entries = timeline(&[speech(0, Speaker::They, "podemos fechar?")], &[]);

        let markdown = render_markdown(&session, &entries, 1_700_000_060_000);

        assert!(markdown.contains("## Resumo"));
        assert!(markdown.contains("Decidimos priorizar o login."));
        // O resumo vem antes da linha temporal.
        let summary_at = markdown.find("## Resumo").expect("resumo");
        let speech_at = markdown.find("Interlocutor").expect("fala");
        assert!(summary_at < speech_at);
    }

    #[test]
    fn omits_an_empty_summary() {
        let markdown = render_markdown(
            &session(Some("Sprint"), Some(1_700_000_060_000)),
            &[],
            1_700_000_060_000,
        );
        assert!(!markdown.contains("## Resumo"));
    }

    #[test]
    fn uses_untitled_and_ongoing_for_a_fresh_session() {
        let markdown = render_markdown(&session(None, None), &[], 1_700_000_120_000);

        assert!(markdown.contains("# Sessão sem título"));
        assert!(markdown.contains("- **Fim:** em andamento"));
        assert!(markdown.contains("- **Duração:** 02:00"));
        assert!(markdown.contains("_Sem falas ou sugestões._"));
    }

    #[test]
    fn keeps_multi_line_text_inside_its_block() {
        let entries = timeline(
            &[speech(0, Speaker::Me, "primeira linha\nsegunda linha")],
            &[suggestion(1_000, "passo 1\npasso 2")],
        );

        let markdown = render_markdown(&session(None, Some(1_700_000_010_000)), &entries, 0);

        assert!(markdown.contains("- **[00:00] Você:** primeira linha\n  segunda linha"));
        assert!(markdown.contains("> 🤖 **Sugestão [00:01]** — passo 1\n> passo 2"));
    }
}
