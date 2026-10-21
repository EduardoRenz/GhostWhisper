/** Tipos e formatação da gravação da sessão (BR-065…BR-071). */
import type { Settings } from "./settings";

/** Estado do botão de câmera. */
export type RecordingStatus = "idle" | "recording" | "paused" | "finalizing";

/** Tipo do artefato. */
export type RecordingKind = "video" | "audio";

/** Estado publicado pelo evento `recording_state`. */
export interface RecordingState {
  status: RecordingStatus;
  kind: RecordingKind | null;
  session_id: string | null;
  target_label: string | null;
  elapsed_ms: number;
  size_bytes: number;
}

/** Estado ocioso. */
export const IDLE_RECORDING: RecordingState = {
  status: "idle",
  kind: null,
  session_id: null,
  target_label: null,
  elapsed_ms: 0,
  size_bytes: 0,
};

/** O ambiente consegue gravar? */
export interface RecordingCapabilities {
  available: boolean;
  encoder: string;
  reason: string | null;
}

/** Resumo da mídia no modal. */
export interface RecordingSummary {
  kind: RecordingKind;
  status: string;
  path: string | null;
  size_bytes: number;
  duration_ms: number;
  width: number;
  height: number;
  fps: number;
  error: string | null;
}

/** Payload do modal "Sessão Finalizada". */
export interface SessionEnd {
  session_id: string;
  subject: string | null;
  started_at_ms: number;
  ended_at_ms: number;
  duration_ms: number;
  transcriptions: number;
  suggestions: number;
  recording: RecordingSummary | null;
}

/** Progresso da finalização. */
export interface RecordingProgress {
  session_id: string;
  phase: "finalizing" | "ready" | "failed";
  percent: number;
  path: string | null;
  size_bytes: number;
  duration_ms: number;
  error: string | null;
}

/** Duração no formato do mockup (`14 min 32 seg`). */
export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  if (hours > 0) {
    return `${hours} h ${String(minutes).padStart(2, "0")} min`;
  }
  if (minutes > 0) {
    return `${minutes} min ${String(seconds).padStart(2, "0")} seg`;
  }
  return `${seconds} seg`;
}

/** Tamanho legível (MB/GB). */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 MB";
  }
  const mb = bytes / (1024 * 1024);
  if (mb < 1) {
    return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  }
  if (mb < 1024) {
    return `${mb.toFixed(1)} MB`;
  }
  return `${(mb / 1024).toFixed(2)} GB`;
}

/** Rótulo do cartão de mídia (`214 MB (MP4 · 1080p · 8 fps)`). */
export function mediaLabel(summary: RecordingSummary | null): string {
  if (!summary || summary.status === "discarded") {
    return "Sem gravação";
  }
  if (summary.kind === "audio") {
    return `${formatBytes(summary.size_bytes)} (FLAC)`;
  }
  const height = summary.height > 0 ? `${summary.height}p` : "—";
  return `${formatBytes(summary.size_bytes)} (MP4 · ${height} · ${summary.fps} fps)`;
}

/** Estimativa de tamanho por hora (card de custo das settings). */
export function estimateSizePerHour(settings: Settings): {
  min: number;
  max: number;
} {
  if (settings.recording_mode === "audio") {
    return { min: 117, max: 117 };
  }
  const base =
    settings.recording_quality === "compact"
      ? 55
      : settings.recording_quality === "sharp"
        ? 210
        : 110;
  const scale = Math.max(1, settings.recording_fps) / 8;
  const audio = Math.round((settings.recording_audio_kbps * 3600) / 8 / 1000);
  return {
    min: Math.round(base * scale * 0.6) + audio,
    max: Math.round(base * scale * 1.6) + audio,
  };
}
