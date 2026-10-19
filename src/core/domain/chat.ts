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
  /** Continuação de uma fala longa (mesma fala, parte seguinte). */
  continues?: boolean;
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

/**
 * Teto de caracteres de uma bolha de fala (≈4–5 linhas no painel).
 *
 * Fala contínua — o Interlocutor falando sem pausas — só fecha o turno no silêncio
 * ou no teto do turno (`max_turn_ms`, 25 s), e a bolha crescia sem fim. O excesso
 * **continua numa bolha seguinte** (`continues`), sem mexer no áudio: a
 * transcrição sai inteira e o corte é só de exibição.
 */
export const MAX_SPEECH_BUBBLE_CHARS = 180;

/**
 * Corta `text` em pedaços de até `limit` caracteres **em fim de palavra**.
 *
 * Usado quando uma única frase não cabe: nunca devolve uma palavra partida (só
 * corta no meio se a "palavra" sozinha já passar do limite, ex.: uma URL).
 */
function chopAtWords(text: string, limit: number): string[] {
  const parts: string[] = [];
  let rest = text.trim();
  while (rest.length > limit) {
    // Janela com um caractere a mais: um espaço exatamente no limite conta.
    const cut = rest.slice(0, limit + 1).lastIndexOf(" ");
    const end = cut > 0 ? cut : limit;
    parts.push(rest.slice(0, end).trim());
    rest = rest.slice(end).trim();
  }
  if (rest.length > 0) {
    parts.push(rest);
  }
  return parts;
}

/**
 * Quebra uma fala em partes de no máximo `limit` caracteres.
 *
 * Prefere o **fim da frase** (`.!?…` ou quebra de linha) e só recorre ao limite de
 * palavra quando a frase sozinha passa do teto — fala curta continua sendo uma
 * bolha só (`["fala"]`), então nada muda no caso comum.
 */
export function splitSpeechText(
  text: string,
  limit: number = MAX_SPEECH_BUBBLE_CHARS,
): string[] {
  const trimmed = text.trim();
  if (trimmed.length === 0) {
    return [];
  }
  if (trimmed.length <= limit) {
    return [trimmed];
  }

  // Frases com o terminador junto (o `\n` também separa).
  const sentences = trimmed.match(/[^.!?…\n]+[.!?…]*\s*|\n+/gu) ?? [trimmed];
  const parts: string[] = [];
  let current = "";
  for (const sentence of sentences) {
    if (current.length > 0 && (current + sentence).trim().length > limit) {
      parts.push(current.trim());
      current = "";
    }
    current += sentence;
    if (current.trim().length > limit) {
      // A frase (ou o trecho sem pontuação) sozinha passou do teto.
      parts.push(...chopAtWords(current, limit));
      current = "";
    }
  }
  if (current.trim().length > 0) {
    parts.push(current.trim());
  }
  return parts.filter((part) => part.length > 0);
}

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
    const text = segment.text.trim();
    if (!hasContent(text)) {
      continue;
    }
    // Fala longa continua na bolha seguinte (continuação da mesma fala).
    splitSpeechText(text).forEach((part, index) => {
      bubbles.push({
        kind: "speech",
        id: index === 0 ? segment.id : `${segment.id}#${index}`,
        speaker: segment.speaker,
        text: part,
        at_ms: segment.start_ms,
        end_ms: segment.end_ms,
        continues: index > 0,
      });
    });
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
    if (entry.kind !== "speech") {
      bubbles.push({
        kind: "suggestion",
        id: `suggestion-${entry.at_ms}-${index}`,
        text,
        at_ms: entry.at_ms,
      });
      return;
    }
    // Mesma regra do ao vivo: fala longa continua na bolha seguinte.
    const id = `speech-${entry.at_ms}-${index}`;
    splitSpeechText(text).forEach((part, partIndex) => {
      bubbles.push({
        kind: "speech",
        id: partIndex === 0 ? id : `${id}#${partIndex}`,
        speaker: entry.speaker,
        text: part,
        at_ms: entry.at_ms,
        continues: partIndex > 0,
      });
    });
  });
  bubbles.sort(byTimeThenKind);
  return bubbles.slice(-cap);
}
