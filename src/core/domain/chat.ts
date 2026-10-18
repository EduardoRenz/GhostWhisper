/**
 * Linha temporal da conversa (puro).
 *
 * Espelha `application/session_timeline.rs` no cliente: falas finalizadas, falas
 * em andamento e sugestões da IA numa **única** ordem de tempo. É o que a UI
 * (ao vivo e histórico) consome para montar as bolhas.
 *
 * Regra que corrige a ISS-018: a fala **em andamento** não disputa posição na
 * linha temporal — ela vive depois do último item, como no Perssua.
 */

import type {
  InsightSuggestion,
  Speaker,
  TimelineEntry,
  TranscriptSegment,
} from "./transcript";

/** Bolha de fala finalizada. */
export interface SpeechBubble {
  kind: "speech";
  id: string;
  speaker: Speaker;
  text: string;
  at_ms: number;
  /** Fim da fala (ms); ausente quando a origem não informa (histórico). */
  end_ms?: number;
}

/** Bolha de sugestão da IA (BR-020). */
export interface SuggestionBubble {
  kind: "suggestion";
  id: string;
  text: string;
  at_ms: number;
  /** Nome do agente (quando conhecido). */
  mode_name?: string;
}

export type ChatBubble = SpeechBubble | SuggestionBubble;

/** Ordem de exibição dos falantes (igual ao Perssua: você antes do outro). */
export const SPEAKER_ORDER: Speaker[] = ["ME", "THEY"];

/** Teto de bolhas em tela (cada bolha é mais pesada que uma linha de texto). */
export const MAX_BUBBLES = 300;

/** Fala em branco (só ruído/pontuação do ASR) não vira bolha. */
const hasContent = (text: string): boolean => text.trim().length > 0;

/**
 * Ordena por tempo; no empate, falas antes de sugestões e, entre iguais, por id
 * (estável e determinístico).
 */
function byTimeThenKind(a: ChatBubble, b: ChatBubble): number {
  if (a.at_ms !== b.at_ms) {
    return a.at_ms - b.at_ms;
  }
  if (a.kind !== b.kind) {
    return a.kind === "speech" ? -1 : 1;
  }
  return a.id.localeCompare(b.id);
}

/**
 * Separa a lista de segmentos entre **finalizados** (linha temporal) e **prévias**
 * (bolhas em andamento).
 *
 * Dedupe por id: prévia e final do mesmo turno compartilham o id e o **final
 * vence** — uma prévia atrasada nunca reescreve o texto já fechado.
 */
export function splitChatTimeline(segments: TranscriptSegment[]): {
  committed: TranscriptSegment[];
  pending: TranscriptSegment[];
} {
  const byId = new Map<string, TranscriptSegment>();
  for (const segment of segments) {
    const existing = byId.get(segment.id);
    if (!existing || segment.final || !existing.final) {
      byId.set(segment.id, segment);
    }
  }

  const committed: TranscriptSegment[] = [];
  // Uma prévia por falante: é uma fala em andamento de cada canal, no máximo.
  const pendingBySpeaker = new Map<Speaker, TranscriptSegment>();
  for (const segment of byId.values()) {
    if (segment.final) {
      committed.push(segment);
      continue;
    }
    // Retratação (ISS-021): um final de texto vazio substitui a prévia do mesmo
    // turno; prévia em branco não vira bolha "em andamento".
    if (segment.text.trim().length === 0) {
      continue;
    }
    const current = pendingBySpeaker.get(segment.speaker);
    if (!current || segment.start_ms >= current.start_ms) {
      pendingBySpeaker.set(segment.speaker, segment);
    }
  }

  committed.sort(
    (a, b) => a.start_ms - b.start_ms || a.id.localeCompare(b.id),
  );

  const pending = SPEAKER_ORDER.map((speaker) =>
    pendingBySpeaker.get(speaker),
  ).filter((segment): segment is TranscriptSegment => segment !== undefined);

  return { committed, pending };
}

/**
 * Monta a conversa completa: falas finalizadas + sugestões na mesma linha do
 * tempo, mais as falas em andamento (fora da linha, exibidas no fim).
 */
export function buildChatTimeline(
  segments: TranscriptSegment[],
  suggestions: InsightSuggestion[] = [],
  cap: number = MAX_BUBBLES,
): { bubbles: ChatBubble[]; pending: TranscriptSegment[] } {
  const { committed, pending } = splitChatTimeline(segments);

  const bubbles: ChatBubble[] = [];
  for (const segment of committed) {
    if (hasContent(segment.text)) {
      bubbles.push({
        kind: "speech",
        id: segment.id,
        speaker: segment.speaker,
        text: segment.text.trim(),
        at_ms: segment.start_ms,
        end_ms: segment.end_ms,
      });
    }
  }

  const seenSuggestions = new Set<string>();
  for (const suggestion of suggestions) {
    // O atalho e o banco devolvem a mesma sugestão: uma bolha só.
    const key =
      suggestion.message_id ?? `suggestion-${suggestion.at_ms}-${suggestion.text}`;
    if (seenSuggestions.has(key)) {
      continue;
    }
    seenSuggestions.add(key);
    if (hasContent(suggestion.text)) {
      bubbles.push({
        kind: "suggestion",
        id: key,
        text: suggestion.text.trim(),
        at_ms: suggestion.at_ms,
        mode_name: suggestion.mode_name,
      });
    }
  }

  bubbles.sort(byTimeThenKind);
  return { bubbles: bubbles.slice(-cap), pending };
}

/**
 * Converte a linha temporal persistida (`db_session_timeline`) em bolhas.
 *
 * Usada pelo detalhe do histórico, que já recebe falas e sugestões fundidas do
 * backend (`application/session_timeline.rs`) — a UI não refaz a fusão.
 */
export function bubblesFromTimeline(
  entries: TimelineEntry[],
  cap: number = MAX_BUBBLES,
): ChatBubble[] {
  const bubbles: ChatBubble[] = [];
  entries.forEach((entry, index) => {
    const text = entry.text.trim();
    if (!hasContent(text)) {
      return;
    }
    bubbles.push(
      entry.kind === "speech"
        ? {
            kind: "speech",
            id: `speech-${entry.at_ms}-${index}`,
            speaker: entry.speaker,
            text,
            at_ms: entry.at_ms,
          }
        : {
            kind: "suggestion",
            id: `suggestion-${entry.at_ms}-${index}`,
            text,
            at_ms: entry.at_ms,
          },
    );
  });
  bubbles.sort(byTimeThenKind);
  return bubbles.slice(-cap);
}
