import type {
  SuggestionClassification,
  SuggestionClassifierState,
} from "../../core/domain/assistant";
import { suggestionCueKey } from "../../core/l10n/labels";
import { useTranslation } from "../../core/l10n";
import { Pill } from "../../components/Pill";
import { SparkleIcon } from "../../components/icons";

interface ClassifierSuggestControlProps {
  paused: boolean;
  sessionActive: boolean;
  classification: SuggestionClassification | null;
  state: SuggestionClassifierState;
  generating: boolean;
  onToggle: () => void;
}

/**
 * Controle do modo classificador no cabeçalho.
 *
 * No modo classificador não existe cronômetro: o estado do bloco/classificação
 * é a informação. O clique continua pausando/retomando o interruptor mestre.
 */
export function ClassifierSuggestControl({
  paused,
  sessionActive,
  classification,
  state,
  generating,
  onToggle,
}: ClassifierSuggestControlProps) {
  const { t } = useTranslation();
  const cue = classification
    ? t(suggestionCueKey(classification.cue))
    : null;

  const label = paused
    ? t("suggestions_paused")
    : !sessionActive
      ? t("suggestions_not_listening")
      : generating
        ? t("suggestions_generating")
        : state === "analyzing"
          ? t("suggestion_classifier_analyzing")
          : state === "listening"
            ? t("suggestion_classifier_listening")
            : state === "error"
              ? t("suggestion_classifier_error")
              : classification?.should_suggest
                ? t("suggestion_classifier_will_suggest", { cue: cue ?? "" })
                : classification
                  ? classification.cue === "no_action"
                    ? t("suggestion_cue_no_action")
                    : classification.cue === "already_answered"
                      ? t("suggestion_cue_already_answered")
                      : classification.cue === "insufficient_context"
                        ? t("suggestion_cue_insufficient_context")
                        : t("suggestion_classifier_nothing", { cue: cue ?? "" })
                  : t("suggestion_classifier_awaiting");

  const title = paused
    ? t("suggestions_automatic_suggestions_paused_click_resume")
    : !sessionActive
      ? t("suggestions_no_listening_session_start_one_notch")
      : t("suggestion_classifier_toggle_title");

  return (
    <button
      type="button"
      className={[
        "classifier-control",
        paused ? "classifier-control--paused" : "",
        state === "analyzing" ? "classifier-control--analyzing" : "",
        state === "error" ? "classifier-control--error" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      onClick={onToggle}
      title={title}
      aria-label={title}
      aria-live="polite"
    >
      <SparkleIcon size={14} />
      <span className="classifier-control__label">
        {t("suggestion_classifier_label")}
      </span>
      <Pill
        className="classifier-control__state"
        tone={
          paused || !sessionActive || state === "error"
            ? "neutral"
            : classification?.should_suggest
              ? "accent"
              : "neutral"
        }
        mono
      >
        {label}
      </Pill>
    </button>
  );
}
