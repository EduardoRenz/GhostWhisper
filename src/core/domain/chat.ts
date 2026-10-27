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
  /** Texto inteiro do segmento (antes do split de exibição — BR-041). */
  source_text?: string;
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
 * Fala contínua — Outros falando sem pausas — só fecha o turno no silêncio
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

/** Fala mínima usada pelo corte de sobreposição (ao vivo e histórico). */
interface SpeechSpan {
  id: string;
  speaker: Speaker;
  text: string;
  start_ms: number;
  end_ms?: number;
}

/**
 * Acha o corte de texto mais próximo de `target`, preferindo fim de frase e,
 * depois, fim de palavra. Nunca devolve um índice fora de `[min, max]`.
 */
function nearestTextCut(
  text: string,
  target: number,
  min: number,
  max: number,
): number {
  if (max <= min) {
    return Math.min(text.length - 1, Math.max(1, target));
  }
  const clamped = Math.min(max, Math.max(min, target));
  let sentence: number | null = null;
  let word: number | null = null;
  for (let index = min; index <= max; index += 1) {
    const previous = text[index - 1] ?? "";
    if (/[.!?…\n]/u.test(previous)) {
      if (sentence === null || Math.abs(index - clamped) < Math.abs(sentence - clamped)) {
        sentence = index;
      }
    }
    if (text[index] === " ") {
      if (word === null || Math.abs(index - clamped) < Math.abs(word - clamped)) {
        word = index;
      }
    }
  }
  return sentence ?? word ?? clamped;
}

/**
 * Divide o texto em `ratios.length + 1` partes nas proporções dadas.
 *
 * Se algum corte produzir parte vazia, devolve o texto inteiro (melhor não
 * intercalar do que perder conteúdo).
 */
function splitTextAtRatios(text: string, ratios: number[]): string[] {
  const trimmed = text.trim();
  if (ratios.length === 0 || trimmed.length === 0) {
    return [trimmed];
  }
  const cuts: number[] = [];
  let previous = 0;
  ratios.forEach((ratio, index) => {
    const remaining = ratios.length - index;
    const target = Math.round(
      trimmed.length * Math.min(0.98, Math.max(0.02, ratio)),
    );
    const min = previous + 1;
    const max = trimmed.length - remaining;
    if (max < min) {
      cuts.push(previous);
      return;
    }
    const cut = nearestTextCut(trimmed, target, min, max);
    cuts.push(cut);
    previous = cut;
  });

  const parts: string[] = [];
  let start = 0;
  for (const cut of cuts) {
    parts.push(trimmed.slice(start, cut).trim());
    start = cut;
  }
  parts.push(trimmed.slice(start).trim());
  return parts.some((part) => part.length === 0) ? [trimmed] : parts;
}

/**
 * Corta falas longas quando o **outro falante começa** dentro do intervalo.
 *
 * Sem timestamps por palavra, o texto é dividido proporcionalmente ao tempo
 * (preferindo fim de frase/palavra). A parte anterior fica no tempo original; a
 * seguinte começa **1 ms depois** do início do outro falante, para a ordenação
 * colocar a nova fala no meio. Cada parte vira uma bolha própria, com rótulo e
 * hora (o resto não é uma "continuação" silenciosa).
 */
function splitSpeechSpans(spans: SpeechSpan[]): SpeechSpan[] {
  const ordered = [...spans].sort(
    (a, b) => a.start_ms - b.start_ms || a.id.localeCompare(b.id),
  );
  const parts: SpeechSpan[] = [];

  for (const span of ordered) {
    const text = span.text.trim();
    if (!hasContent(text)) {
      continue;
    }
    const end = span.end_ms;
    const boundaries =
      end === undefined || end <= span.start_ms
        ? []
        : ordered
            .filter(
              (other) =>
                other.speaker !== span.speaker &&
                other.start_ms > span.start_ms &&
                other.start_ms < end,
            )
            .map((other) => other.start_ms)
            .filter((value, index, all) => all.indexOf(value) === index)
            .sort((a, b) => a - b);

    if (boundaries.length === 0) {
      parts.push({ ...span, text });
      continue;
    }

    const duration = end! - span.start_ms;
    const ratios = boundaries.map((boundary) => (boundary - span.start_ms) / duration);
    const texts = splitTextAtRatios(text, ratios);
    if (texts.length !== boundaries.length + 1) {
      // Não foi possível cortar sem perder texto: mantém a fala inteira.
      parts.push({ ...span, text });
      continue;
    }

    texts.forEach((part, index) => {
      if (!hasContent(part)) {
        return;
      }
      parts.push({
        id: index === 0 ? span.id : `${span.id}#ov${index}`,
        speaker: span.speaker,
        text: part,
        // A parte seguinte ao outro falante entra 1 ms depois dele, para não
        // empatar na ordenação (a fala do outro vem primeiro no empate).
        start_ms: index === 0 ? span.start_ms : boundaries[index - 1]! + 1,
        end_ms: index === texts.length - 1 ? end : boundaries[index],
      });
    });
  }

  return parts;
}

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
  const spans: SpeechSpan[] = committed.map((segment) => ({
    id: segment.id,
    speaker: segment.speaker,
    text: segment.text,
    start_ms: segment.start_ms,
    end_ms: segment.end_ms,
  }));

  // 1) Corta no início da fala do outro canal (linha do tempo intercalada).
  // 2) Depois aplica o teto de exibição de 180 caracteres em cada parte.
  for (const part of splitSpeechSpans(spans)) {
    const text = part.text.trim();
    if (!hasContent(text)) {
      continue;
    }
    splitSpeechText(text).forEach((piece, index) => {
      bubbles.push({
        kind: "speech",
        id: index === 0 ? part.id : `${part.id}#${index}`,
        speaker: part.speaker,
        text: piece,
        at_ms: part.start_ms,
        end_ms: part.end_ms,
        continues: index > 0,
        source_text: text,
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

  // Falas: mesma regra do ao vivo (corte na sobreposição + teto de 180).
  const spans: SpeechSpan[] = [];
  entries.forEach((entry, index) => {
    if (entry.kind !== "speech" || !hasContent(entry.text)) {
      return;
    }
    spans.push({
      id: `speech-${entry.at_ms}-${index}`,
      speaker: entry.speaker,
      text: entry.text,
      start_ms: entry.at_ms,
      end_ms: entry.end_ms,
    });
  });
  for (const part of splitSpeechSpans(spans)) {
    const text = part.text.trim();
    if (!hasContent(text)) {
      continue;
    }
    splitSpeechText(text).forEach((piece, partIndex) => {
      bubbles.push({
        kind: "speech",
        id: partIndex === 0 ? part.id : `${part.id}#${partIndex}`,
        speaker: part.speaker,
        text: piece,
        at_ms: part.start_ms,
        end_ms: part.end_ms,
        continues: partIndex > 0,
        source_text: text,
      });
    });
  }

  entries.forEach((entry, index) => {
    if (entry.kind !== "suggestion" || !hasContent(entry.text)) {
      return;
    }
    bubbles.push({
      kind: "suggestion",
      id: `suggestion-${entry.at_ms}-${index}`,
      text: entry.text.trim(),
      at_ms: entry.at_ms,
    });
  });

  bubbles.sort(byTimeThenKind);
  return bubbles.slice(-cap);
}

/** Linha de renderização: bolha da linha do tempo ou fala em andamento. */
export type ChatRow =
  | { kind: "bubble"; bubble: ChatBubble }
  | { kind: "pending"; segment: TranscriptSegment };

/**
 * Junta as bolhas finalizadas com as falas em andamento **na ordem do tempo**.
 *
 * A fala em andamento tem um `start_ms` fixo; prendê-la no fim fazia a fala do
 * outro canal parecer ter vindo antes (o balão do usuário "passava para frente"
 * enquanto o turno longo do outro lado ainda estava em processamento). A posição
 * é a mesma antes e depois do final, então a bolha não pula ao fechar.
 */
export function buildChatRows(
  bubbles: ChatBubble[],
  pending: TranscriptSegment[] = [],
  cap: number = MAX_BUBBLES,
): ChatRow[] {
  const rows: ChatRow[] = [
    ...bubbles.map((bubble): ChatRow => ({ kind: "bubble", bubble })),
    ...pending.map((segment): ChatRow => ({ kind: "pending", segment })),
  ];
  rows.sort((a, b) => {
    const at = a.kind === "bubble" ? a.bubble.at_ms : a.segment.start_ms;
    const bt = b.kind === "bubble" ? b.bubble.at_ms : b.segment.start_ms;
    if (at !== bt) {
      return at - bt;
    }
    // No mesmo instante, o que já fechou vem antes; entre prévias, ME antes
    // de THEY (mesma ordem do painel).
    if (a.kind !== b.kind) {
      return a.kind === "bubble" ? -1 : 1;
    }
    if (a.kind === "bubble" && b.kind === "bubble") {
      return byTimeThenKind(a.bubble, b.bubble);
    }
    if (a.kind === "pending" && b.kind === "pending") {
      const bySpeaker =
        SPEAKER_ORDER.indexOf(a.segment.speaker) -
        SPEAKER_ORDER.indexOf(b.segment.speaker);
      return bySpeaker || a.segment.id.localeCompare(b.segment.id);
    }
    return 0;
  });
  return rows.slice(-cap);
}
