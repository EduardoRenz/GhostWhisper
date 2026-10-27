import { useState } from "react";
import {
  SUGGESTION_INTERVALS,
  type Settings,
} from "../../core/domain/settings";
import type {
  SuggestionCue,
  SuggestionMode,
} from "../../core/domain/assistant";
import { useSessionState } from "../../core/application/useSessionState";
import { suggestionClassifyNow } from "../../adapters/ipc/suggestions";
import { useTranslation } from "../../core/l10n";
import { suggestionCueKey } from "../../core/l10n/labels";
import { Pill } from "../../components/Pill";
import { ChevronDownIcon, SparkleIcon } from "../../components/icons";
import { ClassifierModelManager } from "./ClassifierModelManager";

interface SuggestionsActivationBlockProps {
  settings: Settings;
  update: <K extends keyof Settings>(field: K, value: Settings[K]) => void;
}

const POSITIVE_CUES: SuggestionCue[] = ["question", "comment"];

type TestState =
  | { kind: "idle" }
  | { kind: "running" }
  | { kind: "ok"; cue: SuggestionCue; confidence: number; backend: string }
  | { kind: "error"; message: string };

/**
 * Bloco “Modo de ativação de sugestões” no topo de Fontes de IA & Modelos.
 *
 * Dois cards (classificador vs. tempo) + avançado recolhido. A configuração
 * vive no rascunho de settings; o rodapé único salva.
 */
export function SuggestionsActivationBlock({
  settings,
  update,
}: SuggestionsActivationBlockProps) {
  const { t } = useTranslation();
  const { state } = useSessionState();
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [test, setTest] = useState<TestState>({ kind: "idle" });

  const mode: SuggestionMode = settings.suggestion_mode;
  const isClassifier = mode === "classifier";
  const isLaya = settings.suggestion_classifier_backend === "laya";
  const modelValue = isLaya ? "laya" : "heuristic";
  const hasModel = settings.suggestion_classifier_model_path.trim().length > 0;
  const modelStatus = !isLaya
    ? t("suggestion_model_status_offline")
    : hasModel
      ? t("suggestion_model_status_configured")
      : t("suggestion_model_status_missing");

  const selectMode = (next: SuggestionMode) => {
    update("suggestion_mode", next);
    if (!settings.suggestion_auto_refresh) {
      update("suggestion_auto_refresh", true);
    }
  };

  const selectModel = (value: string) => {
    if (value === "heuristic") {
      update("suggestion_classifier_backend", "heuristic");
      return;
    }
    update("suggestion_classifier_backend", "laya");
    update("suggestion_classifier_checkpoint", "multilingual");
  };

  const toggleCue = (cue: SuggestionCue) => {
    const current = settings.suggestion_allowed_cues;
    const next = current.includes(cue)
      ? current.filter((item) => item !== cue)
      : [...current, cue];
    update("suggestion_allowed_cues", next);
  };

  const intervalOptions = Array.from(
    new Set([...SUGGESTION_INTERVALS, settings.suggestion_interval_secs]),
  );

  const runTest = async () => {
    if (!state.session_id) {
      setTest({
        kind: "error",
        message: t("suggestion_test_empty"),
      });
      return;
    }
    setTest({ kind: "running" });
    try {
      const outcome = await suggestionClassifyNow(state.session_id);
      if (!outcome) {
        setTest({ kind: "error", message: t("suggestion_test_empty") });
        return;
      }
      setTest({
        kind: "ok",
        cue: outcome.cue,
        confidence: outcome.confidence,
        backend: outcome.backend,
      });
    } catch (error) {
      setTest({
        kind: "error",
        message: error instanceof Error ? error.message : String(error),
      });
    }
  };

  return (
    <section className="settings__section suggestion-activation">
      <header className="suggestion-activation__header">
        <div className="suggestion-activation__heading">
          <span className="suggestion-activation__icon">
            <SparkleIcon size={16} />
          </span>
          <div>
            <h2 className="settings__section-title">
              {t("suggestion_activation_title")}
            </h2>
            <div className="suggestion-activation__subtitle">
              <Pill tone="neutral">{t("suggestion_activation_badge")}</Pill>
              <span>{t("suggestion_activation_subtitle")}</span>
            </div>
          </div>
        </div>
        <label className="settings__toggle">
          <input
            type="checkbox"
            aria-label={t("suggestion_activation_enabled")}
            checked={settings.suggestion_auto_refresh}
            onChange={(event) =>
              update("suggestion_auto_refresh", event.target.checked)
            }
          />
          <span className="settings__toggle-track" aria-hidden="true" />
        </label>
      </header>

      <div className="suggestion-activation__cards">
        <article
          className={`suggestion-activation__card${
            isClassifier ? " suggestion-activation__card--active" : ""
          }`}
          role="radio"
          aria-checked={isClassifier}
          tabIndex={0}
          onClick={() => selectMode("classifier")}
          onKeyDown={(event) => {
            if (event.target !== event.currentTarget) return;
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              selectMode("classifier");
            }
          }}
        >
          <header className="suggestion-activation__card-header">
            <div>
              <h3>{t("suggestion_mode_classifier_title")}</h3>
              <p>{t("suggestion_mode_classifier_subtitle")}</p>
            </div>
            {isClassifier && (
              <Pill tone="accent">{t("suggestion_activation_active")}</Pill>
            )}
          </header>
          <p className="suggestion-activation__description">
            {t("suggestion_mode_classifier_description")}
          </p>

          <label className="settings__field">
            <span>{t("suggestion_model_selected")}</span>
            <select
              value={modelValue}
              onClick={(event) => event.stopPropagation()}
              onChange={(event) => selectModel(event.target.value)}
            >
              <option value="laya">{t("suggestion_model_laya")}</option>
              <option value="heuristic">
                {t("suggestion_model_heuristic")}
              </option>
            </select>
          </label>

          <label className="settings__field">
            <span>{t("suggestion_scope")}</span>
            <select
              value={settings.suggestion_scope}
              onClick={(event) => event.stopPropagation()}
              onChange={(event) =>
                update(
                  "suggestion_scope",
                  event.target.value as "blocks" | "adaptive",
                )
              }
            >
              <option value="blocks">{t("suggestion_scope_blocks")}</option>
              <option value="adaptive">
                {t("suggestion_scope_adaptive")}
              </option>
            </select>
          </label>

          <div className="suggestion-activation__status">
            <span
              className={`suggestion-activation__dot${
                isLaya && !hasModel ? " suggestion-activation__dot--off" : ""
              }`}
            />
            {modelStatus}
          </div>
          {isLaya && (
            <div onClick={(event) => event.stopPropagation()}>
              <ClassifierModelManager
                modelId={settings.suggestion_classifier_model_id}
                checkpoint={settings.suggestion_classifier_checkpoint}
              />
              <p className="suggestion-activation__note">
                {t("suggestion_laya_poc_note")}
              </p>
            </div>
          )}

          <div className="settings__field">
            <span>{t("suggestion_sensitivity")}</span>
            <div className="suggestion-activation__segmented">
              {(["low", "balanced", "high"] as const).map((value) => (
                <button
                  key={value}
                  type="button"
                  className={
                    settings.suggestion_classifier_sensitivity === value
                      ? "is-active"
                      : ""
                  }
                  onClick={(event) => {
                    event.stopPropagation();
                    update("suggestion_classifier_sensitivity", value);
                  }}
                >
                  {t(
                    value === "low"
                      ? "suggestion_sensitivity_low"
                      : value === "balanced"
                        ? "suggestion_sensitivity_balanced"
                        : "suggestion_sensitivity_high",
                  )}
                </button>
              ))}
            </div>
          </div>

          <button
            type="button"
            className="suggestion-activation__advanced-toggle"
            onClick={(event) => {
              event.stopPropagation();
              setAdvancedOpen((previous) => !previous);
            }}
            aria-expanded={advancedOpen}
          >
            {t("suggestion_advanced")}
            <ChevronDownIcon size={14} />
          </button>
        </article>

        <article
          className={`suggestion-activation__card${
            !isClassifier ? " suggestion-activation__card--active" : ""
          }`}
          role="radio"
          aria-checked={!isClassifier}
          tabIndex={0}
          onClick={() => selectMode("auto")}
          onKeyDown={(event) => {
            if (event.target !== event.currentTarget) return;
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              selectMode("auto");
            }
          }}
        >
          <header className="suggestion-activation__card-header">
            <div>
              <h3>{t("suggestion_mode_auto_title")}</h3>
              <p>{t("suggestion_mode_auto_subtitle")}</p>
            </div>
            {!isClassifier && (
              <Pill tone="accent">{t("suggestion_activation_active")}</Pill>
            )}
          </header>
          <p className="suggestion-activation__description">
            {t("suggestion_mode_auto_description")}
          </p>
          <div className="settings__field">
            <span>{t("suggestion_cadence")}</span>
            <div className="suggestion-activation__segmented">
              {intervalOptions.map((seconds) => (
                <button
                  key={seconds}
                  type="button"
                  className={
                    settings.suggestion_interval_secs === seconds
                      ? "is-active"
                      : ""
                  }
                  onClick={(event) => {
                    event.stopPropagation();
                    update("suggestion_interval_secs", seconds);
                  }}
                >
                  {seconds}s
                </button>
              ))}
            </div>
          </div>
          <p className="suggestion-activation__note">
            {t("suggestion_blind_trigger")}
          </p>
        </article>
      </div>

      {advancedOpen && isClassifier && (
        <div className="suggestion-activation__advanced">
          <div className="settings__grid">
            <label className="settings__field">
              <span>{t("suggestion_classifier_cooldown")}</span>
              <input
                type="number"
                min={5}
                max={120}
                value={settings.suggestion_classifier_cooldown_secs}
                onChange={(event) =>
                  update(
                    "suggestion_classifier_cooldown_secs",
                    Number(event.target.value),
                  )
                }
              />
            </label>
            <label className="settings__field">
              <span>{t("suggestion_classifier_timeout")}</span>
              <input
                type="number"
                min={500}
                max={10_000}
                value={settings.suggestion_classifier_timeout_ms}
                onChange={(event) =>
                  update(
                    "suggestion_classifier_timeout_ms",
                    Number(event.target.value),
                  )
                }
              />
            </label>
            <label className="settings__field">
              <span>{t("suggestion_classifier_fallback")}</span>
              <select
                value={settings.suggestion_classifier_fallback}
                onChange={(event) =>
                  update(
                    "suggestion_classifier_fallback",
                    event.target.value as "skip" | "auto",
                  )
                }
              >
                <option value="skip">
                  {t("suggestion_classifier_fallback_skip")}
                </option>
                <option value="auto">
                  {t("suggestion_classifier_fallback_auto")}
                </option>
              </select>
            </label>
            <label className="settings__field">
              <span>{t("suggestion_block_gap")}</span>
              <input
                type="number"
                min={1_000}
                max={10_000}
                value={settings.suggestion_block_gap_ms}
                onChange={(event) =>
                  update("suggestion_block_gap_ms", Number(event.target.value))
                }
              />
            </label>
            <label className="settings__field">
              <span>{t("suggestion_block_max_duration")}</span>
              <input
                type="number"
                min={10_000}
                max={120_000}
                value={settings.suggestion_block_max_ms}
                onChange={(event) =>
                  update("suggestion_block_max_ms", Number(event.target.value))
                }
              />
            </label>
            <label className="settings__field">
              <span>{t("suggestion_block_max_turns")}</span>
              <input
                type="number"
                min={2}
                max={24}
                value={settings.suggestion_block_max_turns}
                onChange={(event) =>
                  update(
                    "suggestion_block_max_turns",
                    Number(event.target.value),
                  )
                }
              />
            </label>
            <label className="settings__field">
              <span>{t("suggestion_model_path")}</span>
              <input
                type="text"
                value={settings.suggestion_classifier_model_path}
                placeholder={t("suggestion_model_path_placeholder")}
                onChange={(event) =>
                  update("suggestion_classifier_model_path", event.target.value)
                }
              />
            </label>
            <label className="settings__field">
              <span>{t("suggestion_model_catalog_url")}</span>
              <input
                type="text"
                value={settings.suggestion_classifier_model_catalog_url}
                onChange={(event) =>
                  update(
                    "suggestion_classifier_model_catalog_url",
                    event.target.value,
                  )
                }
              />
            </label>
            <label className="settings__field">
              <span>{t("suggestion_block_context")}</span>
              <input
                type="number"
                min={8}
                max={20}
                value={settings.suggestion_context_blocks}
                onChange={(event) =>
                  update(
                    "suggestion_context_blocks",
                    Number(event.target.value),
                  )
                }
              />
            </label>
            <label className="settings__field">
              <span>{t("suggestion_answered_questions")}</span>
              <input
                type="number"
                min={0}
                max={20}
                value={settings.suggestion_answered_questions}
                onChange={(event) =>
                  update(
                    "suggestion_answered_questions",
                    Number(event.target.value),
                  )
                }
              />
            </label>
          </div>

          <div className="settings__field">
            <span>{t("suggestion_cues_title")}</span>
            <div className="suggestion-activation__cues">
              {POSITIVE_CUES.map((cue) => (
                <label key={cue} className="settings__toggle-row">
                  <input
                    type="checkbox"
                    checked={settings.suggestion_allowed_cues.includes(cue)}
                    onChange={() => toggleCue(cue)}
                  />
                  <span>{t(suggestionCueKey(cue))}</span>
                </label>
              ))}
            </div>
          </div>

          <div className="settings__grid">
            <label className="settings__toggle-row">
              <input
                type="checkbox"
                checked={settings.suggestion_show_classification}
                onChange={(event) =>
                  update(
                    "suggestion_show_classification",
                    event.target.checked,
                  )
                }
              />
              <span>{t("suggestion_show_last_classification")}</span>
            </label>
          </div>

          <div className="suggestion-activation__test">
            <button
              type="button"
              className="settings__button settings__button--small"
              onClick={() => void runTest()}
              disabled={test.kind === "running" || !state.session_id}
            >
              {t("suggestion_test_classifier")}
            </button>
            {test.kind === "ok" && (
              <span className="suggestion-activation__test-result">
                {t("suggestion_test_result", {
                  cue: t(suggestionCueKey(test.cue)),
                  confidence: Math.round(test.confidence * 100),
                  backend: test.backend,
                })}
              </span>
            )}
            {test.kind === "running" && (
              <span className="suggestion-activation__test-result">
                {t("suggestion_test_running")}
              </span>
            )}
            {test.kind === "error" && (
              <span className="suggestion-activation__test-result suggestion-activation__test-result--error">
                {test.message}
              </span>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
