//! Linha temporal de uma sessão: falas + sugestões da IA, ordenadas no tempo.
//!
//! É a base comum do histórico (lista/detalhe) e do export Markdown — uma única
//! implementação pura, testável sem banco nem UI (BR-016).

use serde::Serialize;

use crate::domain::message::{ConversationMessage, MessageRole};
use crate::domain::transcription::{Speaker, TranscriptSegment};

/// Um item da linha temporal de uma sessão.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimelineEntry {
    /// Milissegundos desde o início da sessão.
    pub at_ms: i64,
    #[serde(flatten)]
    pub kind: TimelineKind,
}

/// O que aconteceu naquele instante.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TimelineKind {
    /// Fala transcrita.
    Speech {
        speaker: Speaker,
        text: String,
        /// Fim da fala (ms desde o início da sessão); a UI usa para cortar a
        /// sobreposição com o outro canal na linha temporal.
        end_ms: i64,
    },
    /// Sugestão pedida à IA.
    Suggestion { text: String },
}

impl TimelineEntry {
    /// Rótulo do falante para exibição/exportação.
    pub fn speaker_label(&self) -> Option<&'static str> {
        match self.kind {
            TimelineKind::Speech { speaker, .. } => Some(match speaker {
                Speaker::Me => "Você",
                Speaker::They => "Interlocutor",
            }),
            TimelineKind::Suggestion { .. } => None,
        }
    }

    /// Texto do item.
    pub fn text(&self) -> &str {
        match &self.kind {
            TimelineKind::Speech { text, .. } => text,
            TimelineKind::Suggestion { text } => text,
        }
    }

    /// É uma sugestão da IA?
    pub fn is_suggestion(&self) -> bool {
        matches!(self.kind, TimelineKind::Suggestion { .. })
    }
}

/// Junta falas e mensagens numa única linha temporal ordenada.
///
/// Só entram segmentos **finais** (prévias são revisáveis e não são
/// persistidas) e mensagens do papel `assistant` (sugestões da IA). Empates são
/// desempatados pela fala primeiro e, depois, pelo texto — para a ordem ser
/// determinística entre execuções (o export tem de ser reproduzível).
pub fn build_timeline(
    segments: &[TranscriptSegment],
    messages: &[ConversationMessage],
) -> Vec<TimelineEntry> {
    let mut entries: Vec<TimelineEntry> = Vec::with_capacity(segments.len() + messages.len());

    for segment in segments.iter().filter(|segment| segment.is_final) {
        entries.push(TimelineEntry {
            at_ms: segment.start_ms,
            kind: TimelineKind::Speech {
                speaker: segment.speaker,
                text: segment.text.clone(),
                end_ms: segment.end_ms,
            },
        });
    }

    for message in messages
        .iter()
        .filter(|message| message.role == MessageRole::Assistant)
    {
        entries.push(TimelineEntry {
            at_ms: message.timestamp_ms,
            kind: TimelineKind::Suggestion {
                text: message.content.clone(),
            },
        });
    }

    entries.sort_by(|left, right| {
        left.at_ms
            .cmp(&right.at_ms)
            .then_with(|| left.is_suggestion().cmp(&right.is_suggestion()))
            .then_with(|| left.text().cmp(right.text()))
    });
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::MessageRole;

    fn speech(
        id: &str,
        at: i64,
        speaker: Speaker,
        text: &str,
        is_final: bool,
    ) -> TranscriptSegment {
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

    fn message(id: &str, at: i64, role: MessageRole, content: &str) -> ConversationMessage {
        ConversationMessage {
            id: id.to_string(),
            session_id: "s1".to_string(),
            role,
            content: content.to_string(),
            timestamp_ms: at,
        }
    }

    #[test]
    fn interleaves_speech_and_suggestions_by_time() {
        let segments = vec![
            speech("1", 0, Speaker::They, "pergunta", true),
            speech("2", 30_000, Speaker::Me, "resposta", true),
        ];
        let messages = vec![
            message("m1", 10_000, MessageRole::Assistant, "sugestão"),
            message("m2", 40_000, MessageRole::Assistant, "outra"),
        ];

        let timeline = build_timeline(&segments, &messages);

        assert_eq!(
            timeline
                .iter()
                .map(|entry| (entry.at_ms, entry.is_suggestion()))
                .collect::<Vec<_>>(),
            vec![(0, false), (10_000, true), (30_000, false), (40_000, true)]
        );
    }

    #[test]
    fn ignores_previews_and_non_assistant_messages() {
        let segments = vec![
            speech("1", 0, Speaker::Me, "prévia", false),
            speech("2", 1_000, Speaker::Me, "final", true),
        ];
        let messages = vec![
            message("m1", 500, MessageRole::User, "eu"),
            message("m2", 600, MessageRole::System, "sistema"),
        ];

        let timeline = build_timeline(&segments, &messages);

        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].text(), "final");
    }

    #[test]
    fn orders_ties_deterministically_with_speech_first() {
        let segments = vec![speech("1", 1_000, Speaker::Me, "fala", true)];
        // Mesmo instante: a fala vem antes da sugestão.
        let messages = vec![message("m1", 1_000, MessageRole::Assistant, "sugestão")];

        let timeline = build_timeline(&segments, &messages);

        assert_eq!(
            timeline
                .iter()
                .map(|entry| entry.is_suggestion())
                .collect::<Vec<_>>(),
            vec![false, true]
        );
    }

    #[test]
    fn labels_the_speaker_for_display() {
        let segments = vec![
            speech("1", 0, Speaker::Me, "eu", true),
            speech("2", 0, Speaker::They, "ele", true),
        ];
        let timeline = build_timeline(&segments, &[]);
        let labels: Vec<_> = timeline
            .iter()
            .filter_map(|entry| entry.speaker_label())
            .collect();
        assert!(labels.contains(&"Você"));
        assert!(labels.contains(&"Interlocutor"));
    }

    /// Contrato com o frontend (`core/domain/transcript.ts`): o TS despacha pelo
    /// campo `kind`, então o JSON precisa manter exatamente este formato.
    #[test]
    fn serializes_to_the_shape_the_frontend_expects() {
        let segments = vec![speech("1", 1_500, Speaker::They, "pergunta", true)];
        let messages = vec![message("m1", 2_000, MessageRole::Assistant, "sugestão")];

        let timeline = build_timeline(&segments, &messages);
        let json: Vec<serde_json::Value> =
            serde_json::from_str(&serde_json::to_string(&timeline).expect("serialize"))
                .expect("parse");

        assert_eq!(json[0]["kind"], "speech");
        assert_eq!(json[0]["at_ms"], 1_500);
        assert_eq!(json[0]["speaker"], "THEY");
        assert_eq!(json[0]["text"], "pergunta");
        assert_eq!(json[1]["kind"], "suggestion");
        assert_eq!(json[1]["text"], "sugestão");
        assert!(json[1].get("speaker").is_none());
    }
}
