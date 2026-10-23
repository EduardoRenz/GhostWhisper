import { useState } from "react";
import { copyText } from "../../core/application/clipboard";
import { useLlmSource } from "../../core/application/useLlmSource";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import { useSuggestions } from "../../core/application/useSuggestions";
import { useManualSuggestionContext } from "../../core/application/useManualSuggestionContext";
import { useSuggestionCadence } from "../../core/application/useSuggestionCadence";
import type { Suggestion } from "../../core/domain/assistant";
import { saveSettings } from "../../adapters/ipc/settings";
import { openSettingsWindow } from "../../adapters/ipc/overlay";
import { useTranslation } from "../../core/l10n";
import { SlidersIcon, SparkleIcon } from "../../components/icons";
import { Pill } from "../../components/Pill";
import { StatusPill } from "../../components/StatusPill";
import { AutoSuggestControl } from "./AutoSuggestControl";
import { ManualContextCard } from "./ManualContextCard";
import { SuggestionCard } from "./SuggestionCard";
import { SuggestionHistory } from "./SuggestionHistory";
import { PanelFrame } from "../panels/PanelFrame";

/**
 * Janela de sugestões.
 *
 * O agente ativo vem do **seletor global do notch** (BR-080): esta janela só
 * consome `settings.assistant_mode_id` e reage ao `settings_changed`.
 */
export function SuggestionsWindow() {
  const { t } = useTranslation();
  const { state } = useSessionState();
  const { settings, reload } = useSettings();
  const cadence = useSuggestionCadence();
  const modeId = settings.assistant_mode_id;
  const [extraContext, setExtraContext] = useState("");
  /** O contexto extra fica recolhido por padrão (abre pelo ícone do header). */
  const [extraOpen, setExtraOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const auto = settings.suggestion_auto_refresh;

  const manual = useManualSuggestionContext({ sessionId: state.session_id });

  // Modelo que respondeu a **sugestão** deste agente (BR-051): o escopo filtra
  // o evento, para não mostrar o modelo do resumo.
  const llmSource = useLlmSource(`agent:${modeId}`);

  const {
    current,
    quick,
    awaitingFull,
    history,
    generating,
    streamingTrigger,
    loading,
    error,
    skipped,
    pinned,
    suggestNow,
    generateManual,
    pin,
  } = useSuggestions({
    sessionId: state.session_id,
    modeId,
    extraContext,
    manualContext: manual.context,
  });

  // O automático vive no core (reage a fala nova, com a janela aberta);
  // aqui só ligamos/desligamos.
  const toggleAuto = () => {
    void saveSettings({
      ...settings,
      suggestion_auto_refresh: !settings.suggestion_auto_refresh,
    }).then(() => reload());
  };

  /** Copia uma sugestão (a atual, ou a de um item do histórico). */
  const onCopy = async (target?: Suggestion | null) => {
    const suggestion = target ?? current;
    if (suggestion && (await copyText(suggestion.text))) {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1_500);
    }
  };

  const status = !state.listening
    ? t("suggestions_no_session")
    : state.paused
      ? t("suggestions_paused")
      : t("suggestions_recording");

  /** Feedback imediato do auto (o LLM pode demorar até o primeiro evento). */
  const autoGenerating = cadence.running && cadence.generating && auto;
  const generatingNow = generating || autoGenerating;

  /** Rótulo do pill de geração (BR-042). */
  const generatingLabel = (() => {
    if (!generatingNow) {
      return null;
    }
    if (autoGenerating) {
      return manual.context
        ? t("suggestions_automatic_suggestion_progress_goes_history")
        : t("suggestions_generating_automatic_suggestion");
    }
    if (!current && !quick) {
      return t("suggestions_generating_suggestion");
    }
    if (streamingTrigger === "manual") {
      return t("suggestions_generating_manual_suggestion");
    }
    if (manual.context && loading) {
      return t("suggestions_waiting_current_suggestion");
    }
    return t("suggestions_generating_new_suggestion");
  })();

  // Cabeçalho: faísca + título + badge de estado (BR-039).
  const liveBadge = state.listening ? (
    <Pill
      className="live-badge"
      tone={state.paused ? "neutral" : "accent"}
      mono
      title={status}
    >
      <span className="live-badge__dot" />
      {state.paused ? t("suggestions_live_badge_paused") : t("suggestions_live_badge_live")}
    </Pill>
  ) : null;

  const actions = (
    <>
      <button
        type="button"
        className={`icon-button${extraOpen ? " icon-button--on" : ""}`}
        onClick={() => setExtraOpen((previous) => !previous)}
        aria-expanded={extraOpen}
        title={t("suggestions_extra_context_optional")}
        aria-label={t("suggestions_extra_context_optional")}
      >
        <SlidersIcon size={16} />
      </button>
      <AutoSuggestControl
        cadence={cadence}
        paused={!auto}
        onToggle={toggleAuto}
      />
      <button
        type="button"
        className="suggest-now"
        disabled={generatingNow || !state.session_id}
        onClick={() => void suggestNow()}
        title={t("suggestions_generate_now")}
      >
        <SparkleIcon size={12} />
        {generatingNow
          ? t("suggestions_waiting_ellipsis")
          : t("suggestions_suggest_now")}
      </button>
    </>
  );

  return (
    <PanelFrame
      label="suggestions"
      title={t("common_suggestions")}
      icon={<SparkleIcon size={16} />}
      titleExtra={liveBadge}
      actions={actions}
    >
      {extraOpen && (
        <label className="panel__extra">
          <span className="panel__extra-label">
            {t("suggestions_extra_context_optional")}
          </span>
          <textarea
            className="panel__textarea"
            placeholder={t("suggestions_extra_context_placeholder")}
            value={extraContext}
            onChange={(event) => setExtraContext(event.target.value)}
          />
        </label>
      )}
      <div className="panel__scroll">
        {manual.context && (
          <ManualContextCard
            context={manual.context}
            startedAtMs={state.started_at_ms}
            generating={generating}
            generated={
              current?.trigger === "manual" &&
              current.source?.speaker === manual.context.speaker &&
              current.source?.at_ms === manual.context.at_ms
            }
            onGenerate={() => void generateManual()}
            onClear={() => void manual.clear()}
          />
        )}

        {error ? (
          <div className="panel__error">
            <p>{error}</p>
            <button
              type="button"
              className="panel__button"
              onClick={() => void openSettingsWindow()}
            >
              {t("panel_open_settings")}
            </button>
          </div>
        ) : !state.session_id ? (
          <div className="suggestion-empty">
            <span className="suggestion-empty__icon">
              <SparkleIcon size={18} />
            </span>
            <p className="suggestion-empty__title">
              {t("suggestions_no_active_listening")}
            </p>
            <p className="suggestion-empty__hint">
              {t("suggestions_start_listening_hint")}
            </p>
          </div>
        ) : current || quick ? (
          <>
            {generatingLabel && (
              <StatusPill
                label={generatingLabel}
                title={t("suggestions_current_stays_title")}
              />
            )}
            <SuggestionCard
              current={current}
              quick={quick}
              pinned={Boolean(pinned)}
              copied={copied}
              awaitingFull={awaitingFull}
              startedAtMs={state.started_at_ms}
              llmSource={llmSource}
              onCopy={() => void onCopy()}
              onPin={() => void pin()}
            />
            {skipped && (
              <p className="panel__hint">
                {t("suggestions_waiting_new_speech")}
              </p>
            )}
          </>
        ) : loading || autoGenerating ? (
          <>
            <StatusPill
              label={generatingLabel ?? t("suggestions_generating_suggestion")}
            />
            <p className="panel__empty">
              {t("suggestions_analyzing_conversation")}
            </p>
          </>
        ) : (
          <div className="suggestion-empty">
            <span className="suggestion-empty__icon">
              <SparkleIcon size={18} />
            </span>
            <p className="suggestion-empty__title">
              {t("suggestions_no_suggestion_yet")}
            </p>
            <p className="suggestion-empty__hint">
              {t("suggestions_empty_hint")}
            </p>
          </div>
        )}

        <SuggestionHistory
          items={history.filter(
            (item) => item.generation_id !== current?.generation_id,
          )}
          pinnedId={pinned}
          startedAtMs={state.started_at_ms}
          onCopy={(suggestion) => void onCopy(suggestion)}
          onPin={(suggestion) => void pin(suggestion)}
        />
      </div>
    </PanelFrame>
  );
}
