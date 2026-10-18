/** Tipos de transcrição/sessão (espelham o core Rust). */

export type Speaker = "ME" | "THEY";

export type SessionChannel = "microphone" | "system";

export interface TranscriptSegment {
  id: string;
  session_id: string;
  speaker: Speaker;
  text: string;
  start_ms: number;
  end_ms: number;
  final: boolean;
}

export interface AudioLevel {
  rms: number;
  peak: number;
}

/** Payload do evento `audio_level` (canal + nível). */
export interface AudioLevelEvent extends AudioLevel {
  channel: SessionChannel;
}

export interface SessionState {
  listening: boolean;
  /** Captura suspensa, mas a sessão segue aberta (BR-019). */
  paused: boolean;
  session_id: string | null;
  channels: SessionChannel[];
}

/** Máximo de segmentos mantidos em tela (evita re-render/reflow excessivo). */
export const MAX_SEGMENTS = 500;

/** Item da linha temporal (espelha `application::session_timeline::TimelineEntry`). */
export type TimelineEntry =
  | { at_ms: number; kind: "speech"; speaker: Speaker; text: string }
  | { at_ms: number; kind: "suggestion"; text: string };

/** Sugestão da IA persistida (`ports::insight_generator::InsightSuggestion`). */
export interface InsightSuggestion {
  message_id: string;
  session_id: string;
  text: string;
  /** Milissegundos desde o início da sessão. */
  at_ms: number;
}

/** Mensagem persistida da sessão (`domain::message::ConversationMessage`). */
export interface ConversationMessage {
  id: string;
  session_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp_ms: number;
}

/** Converte uma mensagem gravada na sugestão exibida. */
export const toSuggestion = (
  message: ConversationMessage,
): InsightSuggestion => ({
  message_id: message.id,
  session_id: message.session_id,
  text: message.content,
  at_ms: message.timestamp_ms,
});

/** Sessão persistida, como o histórico precisa (`domain::session::Session`). */
export interface SessionSummary {
  id: string;
  started_at_ms: number;
  ended_at_ms: number | null;
  subject: string | null;
  status: "active" | "ended";
}

/** Título exibido de uma sessão (o Perssua chama de "sessão sem título"). */
export const sessionTitle = (session: SessionSummary): string =>
  session.subject?.trim() || "Sessão sem título";

/** Data/hora local curta (`dd/mm/aaaa hh:mm`). */
export const formatDateTime = (ms: number): string => {
  const date = new Date(ms);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${pad(date.getDate())}/${pad(date.getMonth() + 1)}/${date.getFullYear()} ${pad(
    date.getHours(),
  )}:${pad(date.getMinutes())}`;
};

/** Duração de uma sessão (usa agora quando ainda está aberta). */
export const sessionDuration = (
  session: SessionSummary,
  nowMs: number = Date.now(),
): string => formatClock((session.ended_at_ms ?? nowMs) - session.started_at_ms);

/** Rótulo do item da linha temporal. */
export const timelineLabel = (entry: TimelineEntry): string =>
  entry.kind === "suggestion" ? "Sugestão da IA" : speakerLabel(entry.speaker);

/** Todos os canais, na ordem de exibição. */
export const ALL_CHANNELS: SessionChannel[] = ["microphone", "system"];

/** Formata milissegundos desde o início da sessão como `mm:ss` (ou `h:mm:ss`). */
export const formatClock = (ms: number): string => {
  const total = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const mm = String(minutes).padStart(2, "0");
  const ss = String(seconds).padStart(2, "0");
  return hours > 0 ? `${hours}:${mm}:${ss}` : `${mm}:${ss}`;
};

/**
 * Insere ou substitui um segmento pelo id, mantendo a ordem de chegada.
 *
 * A prévia e o final do mesmo turno compartilham o id, então o final
 * **substitui** a prévia em vez de duplicar a linha.
 */
export function mergeSegment(
  segments: TranscriptSegment[],
  segment: TranscriptSegment,
  cap: number = MAX_SEGMENTS,
): TranscriptSegment[] {
  const index = segments.findIndex((item) => item.id === segment.id);
  if (index === -1) {
    return [...segments, segment].slice(-cap);
  }
  const next = [...segments];
  next[index] = segment;
  return next;
}

/**
 * Junta o histórico persistido com o que já está ao vivo (BR-019).
 *
 * Os dois lados podem ter o **mesmo id** (um final que acabou de ser gravado é
 * devolvido pelo banco e ainda chega por evento): o `final` ganha da prévia e,
 * em empate, o segmento ao vivo (`live`) vence por ser o mais recente.
 */
export function mergeHistory(
  loaded: TranscriptSegment[],
  live: TranscriptSegment[],
  cap: number = MAX_SEGMENTS,
): TranscriptSegment[] {
  const byId = new Map<string, TranscriptSegment>();
  for (const segment of [...loaded, ...live]) {
    const existing = byId.get(segment.id);
    if (!existing || segment.final || !existing.final) {
      byId.set(segment.id, segment);
    }
  }
  return [...byId.values()]
    .sort((a, b) => a.start_ms - b.start_ms || a.id.localeCompare(b.id))
    .slice(-cap);
}

/** Rótulo legível do falante. */
export const speakerLabel = (speaker: Speaker): string =>
  speaker === "ME" ? "Você" : "Interlocutor";

/** Rótulo legível do canal. */
export const channelLabel = (channel: SessionChannel | null): string => {
  if (channel === "system") return "Sistema";
  if (channel === "microphone") return "Microfone";
  return "Inativo";
};

/** Converte RMS/pico em percentual de barra (0–100). */
export const levelPercent = (level: AudioLevel): number => {
  const value = Math.max(level.rms * 3, level.peak);
  return Math.round(Math.min(1, Math.max(0, value)) * 100);
};
