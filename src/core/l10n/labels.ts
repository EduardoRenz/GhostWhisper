/**
 * Chaves de tradução para rótulos dinâmicos (enums e dados de domínio).
 *
 * Componentes nunca montam chaves por concatenação: estes helpers traduzem o
 * valor de domínio para uma chave snake_case válida do catálogo.
 */

import type { SuggestionCue } from "../domain/assistant";
import type { TranslationKey, TranslationParams } from "./catalog";

/** Função de tradução (o `t` devolvido por `useTranslation`). */
export type Translate = (
  key: TranslationKey,
  params?: TranslationParams,
) => string;

/** Falante ME/THEY. */
export function speakerLabelKey(speaker: "ME" | "THEY"): TranslationKey {
  return speaker === "ME" ? "common_microphone" : "common_others";
}

/** Canal de áudio (`null` = inativo). */
export function channelLabelKey(
  channel: "microphone" | "system" | null,
): TranslationKey {
  if (channel === "system") return "common_system";
  if (channel === "microphone") return "common_microphone";
  return "common_inactive";
}

/** Origem de uma entrada do diário. */
export function journalOriginKey(
  origin: "text" | "screenshot" | "both" | "preset",
): TranslationKey {
  switch (origin) {
    case "screenshot":
      return "journal_origin_screenshot";
    case "both":
      return "journal_origin_both";
    case "preset":
      return "journal_origin_preset";
    default:
      return "journal_origin_text";
  }
}

/** Cue do classificador de sugestões. */
export function suggestionCueKey(cue: SuggestionCue): TranslationKey {
  return `suggestion_cue_${cue}` as TranslationKey;
}

/** Modo de aceleração do ASR. */
export function accelerationKey(
  mode: "auto" | "cpu" | "gpu",
): TranslationKey {
  return `acceleration_${mode}` as TranslationKey;
}

/** Idioma de origem da transcrição. */
export function sourceLanguageKey(
  language: "auto" | "pt" | "en",
): TranslationKey {
  if (language === "pt") return "language_portuguese";
  if (language === "en") return "language_english";
  return "language_auto_detect";
}

/** Idioma de destino da tradução. */
export function targetLanguageKey(
  language: "auto" | "pt" | "en",
): TranslationKey {
  if (language === "pt") return "language_portuguese";
  if (language === "en") return "language_english";
  return "language_auto_no_translate";
}

/** Código de idioma exibido (ex.: botão de tradução da bolha). */
export function languageCodeKey(code: string): TranslationKey | null {
  if (code === "pt") return "language_portuguese";
  if (code === "en") return "language_english";
  return null;
}

/** Ancoragem do notch. */
export function notchAnchorKey(
  anchor: "top_center" | "top_right" | "free",
): TranslationKey {
  return `anchor_${anchor}` as TranslationKey;
}

/** Densidade do notch. */
export function hudDensityKey(
  density: "compact" | "standard" | "expanded",
): TranslationKey {
  return `density_${density}` as TranslationKey;
}

/** Legenda do slider de opacidade (mockup). */
export function opacityLegendKey(source: string): TranslationKey {
  if (source.startsWith("Discreto")) return "opacity_discreet";
  if (source.startsWith("Equilibrado")) return "opacity_balanced";
  return "opacity_opaque";
}

/** Modo da gravação. */
export function recordingModeKey(
  mode: "video" | "audio",
): TranslationKey {
  return mode === "audio" ? "recording_mode_audio" : "recording_mode_video";
}

/** Alvo da gravação. */
export function recordingTargetKey(
  target: "window" | "screen",
): TranslationKey {
  return target === "screen"
    ? "recording_target_screen"
    : "recording_target_window";
}

/** Resolução da gravação. */
export function recordingResolutionKey(
  resolution: "native" | "1280x720" | "854x480",
): TranslationKey {
  if (resolution === "1280x720") return "recording_resolution_1280x720";
  if (resolution === "854x480") return "recording_resolution_854x480";
  return "recording_resolution_native";
}

/** Qualidade da gravação. */
export function recordingQualityKey(
  quality: "compact" | "balanced" | "sharp",
): TranslationKey {
  return `recording_quality_${quality}` as TranslationKey;
}

/** Encoder da gravação. */
export function recordingEncoderKey(
  encoder: "auto" | "x264" | "vaapi",
): TranslationKey {
  return `recording_encoder_${encoder}` as TranslationKey;
}

/** Container da gravação. */
export function recordingContainerKey(
  container: "mp4" | "mkv",
): TranslationKey {
  return `recording_container_${container}` as TranslationKey;
}

/** Motivo de indisponibilidade da gravação, já traduzido (null = desconhecido). */
export function recordingUnavailableKey(
  code: string | null | undefined,
): TranslationKey | null {
  switch (code) {
    case "wayland":
      return "recording_unavailable_wayland";
    case "no_display":
      return "recording_unavailable_no_display";
    case "ffmpeg_missing":
      return "recording_unavailable_ffmpeg_missing";
    case "no_audio":
      return "recording_unavailable_no_audio";
    case "gstreamer_missing":
      return "recording_unavailable_gstreamer_missing";
    default:
      return null;
  }
}

/** Nome completo de um provedor (marca + endpoint custom). */
export function providerLabelKey(
  provider: "openai" | "gemini" | "openrouter" | "deepseek" | "custom",
): TranslationKey {
  return `provider_${provider}` as TranslationKey;
}

/** Nome curto de um provedor. */
export function providerShortLabelKey(
  provider: "openai" | "gemini" | "openrouter" | "deepseek" | "custom",
): TranslationKey {
  if (provider === "gemini") return "provider_gemini";
  if (provider === "custom") return "provider_short_custom";
  return providerLabelKey(provider);
}

/** Rótulo de um ícone do agente. */
export function agentIconKey(icon: string): TranslationKey {
  const known = [
    "sparkle",
    "users",
    "braces",
    "code",
    "layers",
    "book",
    "briefcase",
    "terminal",
  ];
  return known.includes(icon)
    ? (`agent_icon_${icon}` as TranslationKey)
    : "agent_icon_sparkle";
}

/** Rótulo de um atalho global. */
export function shortcutActionKey(
  action:
    | "focus_ask"
    | "capture_screen"
    | "toggle_overlay"
    | "toggle_session"
    | "recording_toggle"
    | "toggle_stealth"
    | "request_insight",
): TranslationKey {
  return `shortcut_${action}` as TranslationKey;
}

/** Aba do painel da sessão. */
export function panelTabKey(
  tab: "transcription" | "summary" | "session",
): TranslationKey {
  return `panel_tab_${tab}` as TranslationKey;
}

/** Tipo do bloco de sugestão. */
export function suggestionKindKey(
  kind: "Código" | "Explicativo",
): TranslationKey {
  return kind === "Código"
    ? "suggestion_kind_code"
    : "suggestion_kind_explanatory";
}

/** Título exibido de uma sessão (subject do usuário ou fallback traduzido). */
export function sessionTitleText(
  session: { subject?: string | null },
  t: Translate,
): string {
  return session.subject?.trim() || t("session_untitled");
}

/** Duração legível (`14 min 32 seg`). */
export function formatDurationText(ms: number, t: Translate): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  if (hours > 0) {
    return t("duration_hours_minutes", {
      hours,
      minutes: String(minutes).padStart(2, "0"),
    });
  }
  if (minutes > 0) {
    return t("duration_minutes_seconds", {
      minutes,
      seconds: String(seconds).padStart(2, "0"),
    });
  }
  return t("duration_seconds", { seconds });
}

/** Tamanho legível (MB/GB). */
export function formatBytesText(bytes: number): string {
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
export function mediaLabelText(
  summary:
    | {
        kind: "video" | "audio";
        status: string;
        size_bytes: number;
        height: number;
        fps: number;
      }
    | null,
  t: Translate,
): string {
  if (!summary || summary.status === "discarded") {
    return t("media_none");
  }
  if (summary.status === "failed") {
    return t("media_failed_segments_preserved");
  }
  const size = formatBytesText(summary.size_bytes);
  if (summary.kind === "audio") {
    return t("media_audio_label", { size });
  }
  const height = summary.height > 0 ? `${summary.height}p` : "—";
  return t("media_video_label", {
    size,
    height,
    fps: summary.fps,
  });
}
