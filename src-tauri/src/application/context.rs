//! Janelas de contexto enviadas ao LLM (funções puras, sem I/O).
//!
//! - [`chunks_after`] alimenta o resumo incremental (cursor → chunks).
//! - [`recent_context`] alimenta a sugestão dos “últimos momentos”.
//!
//! Só entram falas **finalizadas** e com texto útil (prévias são revisáveis).

use crate::domain::transcription::TranscriptSegment;

/// Um pedaço de transcrição pronto para virar resumo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextChunk {
    pub text: String,
    /// Início (ms, relativo à sessão) do primeiro segmento do chunk.
    pub from_ms: i64,
    /// Início do último segmento do chunk — vira o novo cursor.
    pub until_ms: i64,
}

/// Contexto recente de uma sugestão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentContext {
    pub text: String,
    /// Início do último segmento incluído (cursor do próximo pedido).
    pub until_ms: i64,
}

/// Linha no formato aceito pelos prompts (`ME:` / `THEY:`).
fn format_line(segment: &TranscriptSegment) -> String {
    format!("{}: {}", segment.speaker.as_str(), segment.text.trim())
}

/// `true` quando a fala entra no contexto.
fn is_usable(segment: &TranscriptSegment) -> bool {
    segment.is_final && !segment.text.trim().is_empty()
}

/// Falas utilizáveis, ordenadas por tempo (e id, para determinismo).
fn usable_sorted(segments: &[TranscriptSegment]) -> Vec<&TranscriptSegment> {
    let mut usable: Vec<&TranscriptSegment> = segments.iter().filter(|s| is_usable(s)).collect();
    usable.sort_by(|a, b| a.start_ms.cmp(&b.start_ms).then_with(|| a.id.cmp(&b.id)));
    usable
}

/// Trunca uma linha gigante sem quebrar caracteres UTF-8.
fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars().take(max_chars).collect()
}

/// Divide as falas **posteriores ao cursor** em até `max_chunks` chunks.
///
/// `cursor = None` inclui desde o início (primeira geração); `Some(ms)` inclui só
/// falas com `start_ms > ms`. Segmentos que compartilham o `start_ms` nunca são
/// separados entre chunks (o microfone e o sistema podem falar no mesmo
/// instante); o cursor é o `start_ms` do último segmento incluído.
pub fn chunks_after(
    segments: &[TranscriptSegment],
    cursor: Option<i64>,
    max_chars: usize,
    max_chunks: usize,
) -> Vec<ContextChunk> {
    if max_chars == 0 || max_chunks == 0 {
        return Vec::new();
    }

    let pending: Vec<&TranscriptSegment> = usable_sorted(segments)
        .into_iter()
        .filter(|segment| cursor.map_or(true, |cursor| segment.start_ms > cursor))
        .collect();

    let mut chunks = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut chars = 0usize;
    let mut from_ms = 0i64;
    let mut until_ms = 0i64;
    let mut last_start: Option<i64> = None;

    for segment in pending {
        if chunks.len() >= max_chunks {
            break;
        }
        let mut line = format_line(segment);
        if line.chars().count() > max_chars {
            line = truncate_chars(&line, max_chars);
        }
        let line_len = line.chars().count();
        let same_instant = last_start == Some(segment.start_ms);
        if !lines.is_empty() && chars + line_len > max_chars && !same_instant {
            chunks.push(ContextChunk {
                text: lines.join("\n"),
                from_ms,
                until_ms,
            });
            lines.clear();
            chars = 0;
        }
        if lines.is_empty() {
            from_ms = segment.start_ms;
        }
        chars += line_len;
        until_ms = segment.start_ms;
        last_start = Some(segment.start_ms);
        lines.push(line);
    }
    if chunks.len() < max_chunks && !lines.is_empty() {
        chunks.push(ContextChunk {
            text: lines.join("\n"),
            from_ms,
            until_ms,
        });
    }

    chunks
}

/// `true` quando a fala entra no contexto **ao vivo** (final **ou** prévia).
fn is_usable_live(segment: &TranscriptSegment) -> bool {
    !segment.text.trim().is_empty()
}

/// Falas utilizáveis **incluindo prévias**: dedupe por id (o final vence).
///
/// É o que permite a sugestão reagir à fala em andamento, como o Perssua
/// (`questionsRequireFinalizedTranscript: false`), sem perder o texto final
/// quando ele chega.
fn usable_sorted_live(segments: &[TranscriptSegment]) -> Vec<&TranscriptSegment> {
    let mut by_id: std::collections::BTreeMap<&str, &TranscriptSegment> =
        std::collections::BTreeMap::new();
    for segment in segments.iter().filter(|segment| is_usable_live(segment)) {
        match by_id.get(segment.id.as_str()) {
            // Um final nunca é rebaixado por uma prévia atrasada.
            Some(existing) if existing.is_final && !segment.is_final => {}
            _ => {
                by_id.insert(segment.id.as_str(), segment);
            }
        }
    }
    let mut usable: Vec<&TranscriptSegment> = by_id.into_values().collect();
    usable.sort_by(|a, b| a.start_ms.cmp(&b.start_ms).then_with(|| a.id.cmp(&b.id)));
    usable
}

/// Monta o contexto a partir das falas já filtradas (janela + tetos).
fn build_recent(
    mut window: Vec<&TranscriptSegment>,
    max_segments: usize,
    max_chars: usize,
) -> Option<RecentContext> {
    if window.is_empty() {
        return None;
    }
    if window.len() > max_segments {
        window = window.split_off(window.len() - max_segments);
    }

    // Preenche do fim para o começo até o teto de caracteres.
    let mut lines: Vec<String> = Vec::new();
    let mut chars = 0usize;
    for segment in window.iter().rev() {
        let line = format_line(segment);
        let line_len = line.chars().count();
        if !lines.is_empty() && chars + line_len > max_chars {
            break;
        }
        let line = if line_len > max_chars {
            truncate_chars(&line, max_chars)
        } else {
            line
        };
        chars += line.chars().count();
        lines.push(line);
    }
    lines.reverse();
    let until_ms = window.last()?.start_ms;
    Some(RecentContext {
        text: lines.join("\n"),
        until_ms,
    })
}

/// Contexto dos “últimos momentos” da sugestão ao vivo, **incluindo a fala em
/// andamento** (prévias).
///
/// Sem as prévias a sugestão só apareceria depois de o turno fechar (BR-028);
/// um final sempre vence a prévia do mesmo turno. Janela de tempo + tetos de
/// falas e de caracteres; `None` quando não há fala utilizável.
pub fn recent_context_live(
    segments: &[TranscriptSegment],
    now_rel_ms: i64,
    window_ms: i64,
    max_segments: usize,
    max_chars: usize,
) -> Option<RecentContext> {
    let cutoff = now_rel_ms.saturating_sub(window_ms.max(0));
    recent_context_from(segments, cutoff, max_segments, max_chars)
}

/// Contexto **incremental**: falas desde `since_ms` (o instante da última
/// pergunta respondida), sem olhar o que já foi respondido.
///
/// A janela de tempo continua valendo como teto: `since_ms` nunca recua além
/// de `now_rel_ms - window_ms` (uma pergunta depois de uma pausa longa não
/// manda meia hora de transcrição).
pub fn context_since(
    segments: &[TranscriptSegment],
    since_ms: i64,
    now_rel_ms: i64,
    window_ms: i64,
    max_segments: usize,
    max_chars: usize,
) -> Option<RecentContext> {
    let window_start = now_rel_ms.saturating_sub(window_ms.max(0));
    recent_context_from(
        segments,
        since_ms.max(window_start),
        max_segments,
        max_chars,
    )
}

/// Falas a partir de `cutoff_ms` (inclusive), com os tetos de sempre.
fn recent_context_from(
    segments: &[TranscriptSegment],
    cutoff_ms: i64,
    max_segments: usize,
    max_chars: usize,
) -> Option<RecentContext> {
    if max_segments == 0 || max_chars == 0 {
        return None;
    }
    let window: Vec<&TranscriptSegment> = usable_sorted_live(segments)
        .into_iter()
        .filter(|segment| segment.start_ms >= cutoff_ms)
        .collect();
    build_recent(window, max_segments, max_chars)
}

/// Início do último segmento final utilizável (para saber se há conteúdo novo).
pub fn last_final_ms(segments: &[TranscriptSegment]) -> Option<i64> {
    usable_sorted(segments)
        .last()
        .map(|segment| segment.start_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::transcription::Speaker;

    fn seg(id: &str, at: i64, speaker: Speaker, text: &str, is_final: bool) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            session_id: "s1".to_string(),
            speaker,
            text: text.to_string(),
            start_ms: at,
            end_ms: at + 500,
            is_final,
        }
    }

    #[test]
    fn skips_previews_and_empty_text() {
        let segments = vec![
            seg("1", 0, Speaker::Me, "prévia", false),
            seg("2", 1_000, Speaker::Me, "  ", true),
            seg("3", 2_000, Speaker::They, "final", true),
        ];
        let chunks = chunks_after(&segments, None, 1_000, 3);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "THEY: final");
        assert_eq!(chunks[0].from_ms, 2_000);
        assert_eq!(chunks[0].until_ms, 2_000);
    }

    #[test]
    fn includes_the_first_turn_at_zero() {
        let segments = vec![seg("1", 0, Speaker::Me, "primeira", true)];
        let chunks = chunks_after(&segments, None, 1_000, 3);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("primeira"));
    }

    #[test]
    fn chunks_advance_the_cursor_and_respect_max_chunks() {
        let (text_a, text_b, text_c) = ("a".repeat(40), "b".repeat(40), "c".repeat(40));
        let segments = vec![
            seg("1", 1_000, Speaker::Me, &text_a, true),
            seg("2", 2_000, Speaker::Me, &text_b, true),
            seg("3", 3_000, Speaker::Me, &text_c, true),
        ];
        let chunks = chunks_after(&segments, None, 45, 2);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].until_ms, 1_000);
        assert_eq!(chunks[1].until_ms, 2_000);

        // Continua do cursor: só o terceiro.
        let rest = chunks_after(&segments, Some(2_000), 45, 2);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].until_ms, 3_000);
        assert!(rest[0].text.contains(&text_c));
    }

    #[test]
    fn never_splits_segments_with_the_same_start() {
        // Mic e sistema no mesmo instante: os dois ficam no mesmo chunk.
        let segments = vec![
            seg("a", 1_000, Speaker::Me, &"x".repeat(30), true),
            seg("b", 1_000, Speaker::They, &"y".repeat(30), true),
            seg("c", 2_000, Speaker::Me, "depois", true),
        ];
        let chunks = chunks_after(&segments, None, 40, 3);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].text.contains("ME: "));
        assert!(chunks[0].text.contains("THEY: "));
        assert_eq!(chunks[0].until_ms, 1_000);
    }

    #[test]
    fn live_context_takes_the_last_moments() {
        let segments = vec![
            seg("1", 1_000, Speaker::They, "antigo", true),
            seg("2", 60_000, Speaker::They, "pergunta técnica", true),
            seg("3", 61_000, Speaker::Me, "resposta", true),
        ];
        let context = recent_context_live(&segments, 62_000, 5_000, 40, 1_000).expect("contexto");
        assert!(!context.text.contains("antigo"));
        assert!(context.text.contains("pergunta técnica"));
        assert!(context.text.contains("ME: resposta"));
        assert_eq!(context.until_ms, 61_000);
    }

    #[test]
    fn live_context_prefers_the_tail_within_the_char_budget() {
        let segments = vec![
            seg("1", 1_000, Speaker::Me, &"a".repeat(200), true),
            seg("2", 2_000, Speaker::Me, "falta recente", true),
        ];
        let context = recent_context_live(&segments, 3_000, 10_000, 40, 30).expect("contexto");
        assert!(context.text.contains("falta recente"));
        assert!(!context.text.contains(&"a".repeat(200)));
    }

    #[test]
    fn live_context_includes_the_speech_in_progress() {
        let segments = vec![
            seg("1", 1_000, Speaker::They, "pergunta", true),
            seg("2", 2_000, Speaker::Me, "estou respon", false),
        ];
        let live = recent_context_live(&segments, 3_000, 10_000, 40, 1_000).expect("contexto");
        assert!(live.text.contains("THEY: pergunta"));
        assert!(live.text.contains("ME: estou respon"));
    }

    #[test]
    fn live_context_prefers_the_final_over_a_stale_preview() {
        let segments = vec![
            seg("t0", 1_000, Speaker::Me, "prévia errada", false),
            seg("t0", 1_000, Speaker::Me, "texto final", true),
        ];
        let live = recent_context_live(&segments, 2_000, 10_000, 40, 1_000).expect("contexto");
        assert!(live.text.contains("ME: texto final"));
        assert!(!live.text.contains("prévia errada"));
    }

    #[test]
    fn live_context_is_none_without_usable_speech() {
        // Sem fala nenhuma (texto vazio) ⇒ nada para sugerir.
        let segments = vec![seg("1", 0, Speaker::Me, "   ", false)];
        assert!(recent_context_live(&segments, 1_000, 5_000, 40, 100).is_none());
        assert!(recent_context_live(&[], 1_000, 5_000, 40, 100).is_none());
    }

    #[test]
    fn context_since_skips_what_was_already_answered() {
        let segments = vec![
            seg("1", 1_000, Speaker::They, "já foi respondido", true),
            seg("2", 10_000, Speaker::They, "pergunta nova", true),
            seg("3", 11_000, Speaker::Me, "resposta", true),
        ];
        // A última pergunta respondida foi feita em 5 s: só o que veio depois.
        let context =
            context_since(&segments, 5_000, 12_000, 180_000, 40, 1_000).expect("contexto");
        assert!(!context.text.contains("já foi respondido"));
        assert!(context.text.contains("pergunta nova"));
        assert!(context.text.contains("ME: resposta"));
    }

    #[test]
    fn context_since_never_goes_further_back_than_the_window() {
        let segments = vec![
            seg("1", 1_000, Speaker::They, "muito antigo", true),
            seg("2", 20_000, Speaker::They, "recente", true),
        ];
        // O corte pedido é 0 ms, mas a janela de 10 s manda só o recente.
        let context = context_since(&segments, 0, 21_000, 10_000, 40, 1_000).expect("contexto");
        assert!(!context.text.contains("muito antigo"));
        assert!(context.text.contains("recente"));
    }

    #[test]
    fn context_since_is_none_when_nothing_new_happened() {
        let segments = vec![seg("1", 1_000, Speaker::They, "antigo", true)];
        assert!(context_since(&segments, 10_000, 12_000, 180_000, 40, 1_000).is_none());
    }

    #[test]
    fn last_final_ms_finds_the_latest_final() {
        let segments = vec![
            seg("1", 1_000, Speaker::Me, "a", true),
            seg("2", 2_000, Speaker::Me, "prévia", false),
            seg("3", 3_000, Speaker::They, "b", true),
        ];
        assert_eq!(last_final_ms(&segments), Some(3_000));
        assert_eq!(last_final_ms(&[]), None);
    }
}
