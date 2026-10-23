import type { SuggestionCadence } from "../../core/application/useSuggestionCadence";
import { useTranslation } from "../../core/l10n";
import { Pill } from "../../components/Pill";

interface AutoSuggestControlProps {
  /** Cadência publicada pelo core (BR-039). */
  cadence: SuggestionCadence;
  /** Liga/desliga o automático (grava nas settings). */
  onToggle: () => void;
  /** `true` enquanto o automático está desligado (o rótulo vira "Pausado"). */
  paused: boolean;
}

/** Raio do anel (viewBox 24). */
const RADIUS = 9;
/** Perímetro do anel (2πr). */
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

/**
 * Controle do auto-sugerir: anel de progresso + rótulo + contagem regressiva.
 *
 * O anel esvazia conforme a próxima sugestão se aproxima e a contagem mostra
 * quanto falta. O texto (“Auto-sugerir”) fica sempre visível; o clique pausa
 * ou retoma o automático.
 */
export function AutoSuggestControl({
  cadence,
  onToggle,
  paused,
}: AutoSuggestControlProps) {
  const { t } = useTranslation();
  const fraction =
    cadence.window_ms > 0
      ? Math.min(1, Math.max(0, cadence.remaining_ms / cadence.window_ms))
      : 0;
  const offset = CIRCUMFERENCE * (1 - fraction);
  const seconds = Math.max(0, Math.ceil(cadence.remaining_ms / 1000));
  const generating = !paused && cadence.running && cadence.generating;
  const noSession = !paused && !cadence.running;
  const waiting = !paused && cadence.running && cadence.awaiting_speech;
  const count = paused
    ? t("suggestions_paused")
    : noSession
      ? t("suggestions_not_listening")
      : generating
        ? t("suggestions_generating")
        : waiting
          ? t("suggestions_waiting")
          : `${seconds}s`;
  // Enquanto gera, o anel vira um arco girando (progresso indeterminado).
  const dashArray = generating
    ? `${CIRCUMFERENCE * 0.28} ${CIRCUMFERENCE}`
    : CIRCUMFERENCE;

  const intervalSeconds = Math.max(1, Math.round(cadence.window_ms / 1000));
  const title = paused
    ? t("suggestions_automatic_suggestions_paused_click_resume")
    : noSession
      ? t("suggestions_no_listening_session_start_one_notch")
      : generating
        ? t("suggestions_generating_automatic_suggestion_now")
        : waiting
          ? t("suggestions_waiting_new_speech_suggest")
          : t("suggestions_automatic_suggestions_every_seconds_s_click", {
              seconds: intervalSeconds,
            });

  return (
    <button
      type="button"
      className={[
        "auto-control",
        paused ? "auto-control--paused" : "",
        waiting ? "auto-control--waiting" : "",
        generating ? "auto-control--generating" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      onClick={onToggle}
      title={title}
      aria-label={title}
    >
      <svg className="auto-control__ring" viewBox="0 0 24 24" aria-hidden="true">
        <circle
          className="auto-control__track"
          cx="12"
          cy="12"
          r={RADIUS}
          fill="none"
        />
        <circle
          className="auto-control__progress"
          cx="12"
          cy="12"
          r={RADIUS}
          fill="none"
          strokeDasharray={dashArray}
          strokeDashoffset={generating ? 0 : offset}
          transform="rotate(-90 12 12)"
        />
      </svg>
      <span className="auto-control__label">{t("suggestions_auto_suggest")}</span>
      <Pill
        className="auto-control__count"
        tone={paused || noSession || waiting || generating ? "neutral" : "accent"}
        mono
      >
        {count}
      </Pill>
    </button>
  );
}
