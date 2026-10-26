import { useState } from "react";
import type {
  SuggestionClassification,
  SuggestionClassifierState,
  SuggestionMode,
} from "../../core/domain/assistant";
import { suggestionCueKey } from "../../core/l10n/labels";
import { useTranslation } from "../../core/l10n";
import { Pill } from "../../components/Pill";
import { ChevronDownIcon } from "../../components/icons";

interface LastClassificationStripProps {
  mode: SuggestionMode;
  classification: SuggestionClassification | null;
  state: SuggestionClassifierState;
  error: string | null;
  canClassify: boolean;
  onClassifyNow: () => void;
  onRetry: () => void;
  onOpenSettings: () => void;
}

function formatOffset(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

function relativeTime(
  elapsedMs: number,
  t: ReturnType<typeof useTranslation>["t"],
): string {
  const seconds = Math.max(0, Math.floor(elapsedMs / 1000));
  if (seconds < 60) {
    return t("suggestions_seconds_s_ago", { seconds });
  }
  return t("suggestions_minutes_min_ago", { minutes: Math.floor(seconds / 60) });
}

/**
 * Faixa persistente da última classificação (BR-095).
 *
 * Sempre renderizada (por padrão), inclusive quando a decisão foi negativa.
 * Não é um segundo cartão de sugestão: é diagnóstico da decisão do core.
 */
export function LastClassificationStrip({
  mode,
  classification,
  state,
  error,
  canClassify,
  onClassifyNow,
  onRetry,
  onOpenSettings,
}: LastClassificationStripProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  if (mode === "auto") {
    return (
      <div className="classification-strip classification-strip--muted">
        <span className="classification-strip__title">
          {t("suggestion_last_classification")}
        </span>
        <span className="classification-strip__hint">
          {t("suggestion_last_classification_auto")}
        </span>
        <button
          type="button"
          className="classification-strip__link"
          onClick={onOpenSettings}
        >
          {t("suggestion_adjust_settings")}
        </button>
      </div>
    );
  }

  if (error && !classification) {
    return (
      <div className="classification-strip classification-strip--error">
        <span className="classification-strip__title">
          {t("suggestion_last_classification")}
        </span>
        <span className="classification-strip__hint">
          {t("suggestion_last_classification_error", { error })}
        </span>
        <button
          type="button"
          className="classification-strip__link"
          onClick={onRetry}
        >
          {t("suggestion_retry")}
        </button>
      </div>
    );
  }

  if (!classification) {
    return (
      <div className="classification-strip classification-strip--empty">
        <span className="classification-strip__title">
          {t("suggestion_last_classification")}
        </span>
        <span className="classification-strip__hint">
          {t("suggestion_last_classification_empty")}
        </span>
        <button
          type="button"
          className="classification-strip__link"
          onClick={onClassifyNow}
          disabled={!canClassify || state === "analyzing"}
        >
          {t("suggestion_classify_now")}
        </button>
      </div>
    );
  }

  const cue = t(suggestionCueKey(classification.cue));
  const decision = classification.should_suggest
    ? t("suggestion_classification_yes")
    : t("suggestion_classification_no");
  const confidence = Math.round(classification.confidence * 100);
  const relative = relativeTime(Date.now() - classification.classified_at_ms, t);
  const block = t("suggestion_classification_block_value", {
    start: formatOffset(classification.block_start_ms),
    end: formatOffset(classification.block_end_ms),
    count: classification.turn_count,
  });

  return (
    <div className="classification-strip">
      <button
        type="button"
        className="classification-strip__summary"
        onClick={() => setOpen((previous) => !previous)}
        aria-expanded={open}
      >
        <span className="classification-strip__title">
          {t("suggestion_last_classification")}
        </span>
        <Pill
          className="classification-strip__cue"
          tone={classification.should_suggest ? "accent" : "neutral"}
        >
          {cue}
        </Pill>
        <span className="classification-strip__decision">
          {error ? `${decision} · ${t("suggestion_classification_stale")}` : decision}
        </span>
        <span className="classification-strip__confidence">
          {confidence}%
        </span>
        <span className="classification-strip__time">{relative}</span>
        <ChevronDownIcon
          size={14}
          className={open ? "classification-strip__chevron--open" : ""}
        />
      </button>

      {open && (
        <div className="classification-strip__details">
          <div className="classification-strip__row">
            <span>{t("suggestion_classification_evidence")}</span>
            <p>{classification.evidence || "—"}</p>
          </div>
          <div className="classification-strip__row">
            <span>{t("suggestion_classification_block")}</span>
            <p>{block}</p>
          </div>
          <div className="classification-strip__row">
            <span>{t("suggestion_classification_backend")}</span>
            <p>
              {classification.backend}
              {classification.model ? ` · ${classification.model}` : ""}
            </p>
          </div>
          <div className="classification-strip__row">
            <span>{t("suggestion_classification_latency")}</span>
            <p>
              {t("suggestion_classification_latency_value", {
                ms: classification.latency_ms,
              })}
            </p>
          </div>
          <div className="classification-strip__actions">
            <button
              type="button"
              className="classification-strip__link"
              onClick={onClassifyNow}
              disabled={!canClassify || state === "analyzing"}
            >
              {t("suggestion_classify_now")}
            </button>
            <button
              type="button"
              className="classification-strip__link"
              onClick={onOpenSettings}
            >
              {t("suggestion_adjust_settings")}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
