/**
 * Linha temporal da conversa (puro).
 *
 * Espelha `application/session_timeline.rs` no cliente: falas finalizadas, falas
 * em andamento e sugestões da IA numa **única** ordem de tempo. É o que a UI
 * (ao vivo e histórico) consome para montar as bolhas.
 *
 * A fala **em andamento** disputa posição pelo `start_ms` (ISS-018/ISS-082) e
 * também serve de fronteira: se ela começou no meio de um bloco finalizado, o
 * bloco é dividido para ela aparecer no lugar certo — não depois dele.
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

/**
 * Limiares do corte de sobreposição (BR-043, revisado pela ISS-075).
 *
 * O corte existe para intercalar quem falou no meio — mas partir qualquer fala
 * em qualquer início do outro canal produzia bolhas de 1–5 caracteres ("e",
 * "vez.", "Ei,"). Só vale a pena dividir uma fala **longa** por uma
 * **interrupção real**, e só se todas as partes continuarem legíveis.
 */
const MIN_OVERLAP_SPEECH_CHARS = 80;
const MIN_OVERLAP_SPEECH_MS = 6_000;
const MIN_OVERLAP_INTERRUPTION_MS = 1_500;
const MIN_OVERLAP_PART_CHARS = 20;
const MIN_OVERLAP_PART_WORDS = 3;

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
 * hora.
 *
 * O corte é **conservador** (ISS-075): só uma fala longa é dividida, só por uma
 * interrupção com duração real e só quando todas as partes ficam legíveis. Antes,
 * qualquer início do outro canal fatiado em pedaços de 1–5 caracteres deixava a
 * conversa cheia de bolhas curtas e sem contexto.
 *
 * As **prévias** (`interruptions`) entram como fronteira, mas não são divididas:
 * a fala em andamento continua sendo uma bolha só (ISS-082).
 */
function splitSpeechSpans(
  spans: SpeechSpan[],
  interruptions: SpeechSpan[] = [],
): SpeechSpan[] {
  const ordered = [...spans].sort(
    (a, b) => a.start_ms - b.start_ms || a.id.localeCompare(b.id),
  );
  // Prévias também interrompem: a fala em andamento tem de entrar no meio do
  // bloco finalizado que ela interrompeu, não depois dele (ISS-082).
  const candidates = [...spans, ...interruptions];
  const parts: SpeechSpan[] = [];

  for (const span of ordered) {
    const text = span.text.trim();
    if (!hasContent(text)) {
      continue;
    }
    const end = span.end_ms;
    const duration =
      end === undefined || end <= span.start_ms ? 0 : end - span.start_ms;
    const boundaries =
      duration === 0
        ? []
        : candidates
            .filter((other) => {
              if (
                other.speaker === span.speaker ||
                other.start_ms <= span.start_ms ||
                other.start_ms >= end!
              ) {
                return false;
              }
              // Interjeição curta não intercala: só uma interrupção real parte
              // a fala (sem `end_ms` não dá para saber a duração).
              const otherEnd = other.end_ms;
              return (
                otherEnd !== undefined &&
                otherEnd - other.start_ms >= MIN_OVERLAP_INTERRUPTION_MS
              );
            })
            .map((other) => other.start_ms)
            .filter((value, index, all) => all.indexOf(value) === index)
            .sort((a, b) => a - b);

    if (
      duration === 0 ||
      text.length < MIN_OVERLAP_SPEECH_CHARS ||
      duration < MIN_OVERLAP_SPEECH_MS ||
      boundaries.length === 0
    ) {
      parts.push({ ...span, text });
      continue;
    }

    const ratios = boundaries.map((boundary) => (boundary - span.start_ms) / duration);
    const texts = splitTextAtRatios(text, ratios);
    const hasTinyPart = texts.some((part) => {
      const trimmed = part.trim();
      return (
        trimmed.length < MIN_OVERLAP_PART_CHARS ||
        trimmed.split(/\s+/u).filter(Boolean).length < MIN_OVERLAP_PART_WORDS
      );
    });
    if (texts.length !== boundaries.length + 1 || hasTinyPart) {
      // O corte deixaria um pedaço sem contexto (ex.: "vez." ou "e"): mantém a
      // fala inteira. Melhor uma bolha longa do que duas sem sentido.
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
  // A prévia ainda não fechou, mas já é uma fala de outro canal: ela serve de
  // fronteira para o bloco finalizado (ISS-082).
  const pendingSpans: SpeechSpan[] = pending.map((segment) => ({
    id: segment.id,
    speaker: segment.speaker,
    text: segment.text,
    start_ms: segment.start_ms,
    end_ms: segment.end_ms,
  }));

  // Uma fala finalizada é **uma** bolha: nada é cortado por tamanho (BR-102).
  // O único corte é o da sobreposição, que intercala quem falou no meio (CA-14).
  for (const part of splitSpeechSpans(spans, pendingSpans)) {
    const text = part.text.trim();
    if (!hasContent(text)) {
      continue;
    }
    bubbles.push({
      kind: "speech",
      id: part.id,
      speaker: part.speaker,
      text,
      at_ms: part.start_ms,
      end_ms: part.end_ms,
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

  // Falas: mesma regra do ao vivo (uma bolha por fala; só a sobreposição divide).
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
    bubbles.push({
      kind: "speech",
      id: part.id,
      speaker: part.speaker,
      text,
      at_ms: part.start_ms,
      end_ms: part.end_ms,
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
