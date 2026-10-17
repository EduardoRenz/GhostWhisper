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
  session_id: string | null;
  channels: SessionChannel[];
}

/** Todos os canais, na ordem de exibição. */
export const ALL_CHANNELS: SessionChannel[] = ["microphone", "system"];

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
