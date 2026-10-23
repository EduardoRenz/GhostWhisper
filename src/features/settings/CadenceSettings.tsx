/**
 * Seção "Cadências & Perguntas Prontas" da aba "Geral & HUD" (BR-074).
 *
 * Saiu da aba "Assistentes" (que agora é só o painel de agentes): as
 * atualizações automáticas de resumo/sugestão e as perguntas prontas da busca
 * são configuração geral, não do agente.
 */

import {
  DEFAULT_SAVED_QUESTIONS,
  MAX_SAVED_QUESTIONS,
  MAX_SAVED_QUESTION_CHARS,
  SUGGESTION_INTERVALS,
  SUMMARY_INTERVALS,
  type SavedQuestion,
  type Settings,
} from "../../core/domain/settings";
import { useTranslation } from "../../core/l10n";

interface CadenceSettingsProps {
  settings: Settings;
  update: <K extends keyof Settings>(field: K, value: Settings[K]) => void;
}

/** Cadências automáticas + perguntas prontas. */
export function CadenceSettings({ settings, update }: CadenceSettingsProps) {
  const { t } = useTranslation();
  const savedQuestions = settings.saved_questions ?? DEFAULT_SAVED_QUESTIONS;
  const setSavedQuestions = (questions: SavedQuestion[]) =>
    update("saved_questions", questions);

  return (
    <section className="settings__section">
      <header className="settings__section-header">
        <h2 className="settings__section-title">
          {t("cadence_cadence_saved_questions")}
        </h2>
        <span className="settings__section-note">
          {t("cadence_automatic_updates_search_chips")}
        </span>
      </header>

      <div className="settings__grid">
        <article className="settings__card settings__card--hud">
          <header className="settings__card-header">
            <h3 className="settings__card-title">
              {t("cadence_automatic_summary")}
            </h3>
            <label className="settings__toggle">
              <input
                type="checkbox"
                aria-label={t("cadence_automatic_summary")}
                checked={settings.summary_auto_refresh}
                onChange={(event) =>
                  update("summary_auto_refresh", event.target.checked)
                }
              />
              <span className="settings__toggle-track" aria-hidden="true" />
            </label>
          </header>
          <p className="settings__card-description">
            {t("cadence_updates_live_session_summary_while_window",
            )}
          </p>
          <label className="settings__field">
            <span>{t("cadence_summary_interval")}</span>
            <select
              value={settings.summary_interval_secs}
              disabled={!settings.summary_auto_refresh}
              onChange={(event) =>
                update("summary_interval_secs", Number(event.target.value))
              }
            >
              {SUMMARY_INTERVALS.map((seconds) => (
                <option key={seconds} value={seconds}>
                  {seconds}s
                </option>
              ))}
            </select>
          </label>
        </article>

        <article className="settings__card settings__card--hud">
          <header className="settings__card-header">
            <h3 className="settings__card-title">
              {t("cadence_automatic_suggestions")}
            </h3>
            <label className="settings__toggle">
              <input
                type="checkbox"
                aria-label={t("cadence_automatic_suggestions")}
                checked={settings.suggestion_auto_refresh}
                onChange={(event) =>
                  update("suggestion_auto_refresh", event.target.checked)
                }
              />
              <span className="settings__toggle-track" aria-hidden="true" />
            </label>
          </header>
          <p className="settings__card-description">
            {t("cadence_generates_new_suggestion_every_interval_without")}
          </p>
          <label className="settings__field">
            <span>{t("cadence_suggestion_interval")}</span>
            <select
              value={settings.suggestion_interval_secs}
              disabled={!settings.suggestion_auto_refresh}
              onChange={(event) =>
                update("suggestion_interval_secs", Number(event.target.value))
              }
            >
              {SUGGESTION_INTERVALS.map((seconds) => (
                <option key={seconds} value={seconds}>
                  {seconds}s
                </option>
              ))}
            </select>
          </label>
        </article>
      </div>

      <article className="settings__card settings__card--hud">
        <header className="settings__card-header">
          <h3 className="settings__card-title">{t("cadence_saved_questions")}</h3>
          <span className="settings__badge">
            {savedQuestions.length}/{MAX_SAVED_QUESTIONS}
          </span>
        </header>
        <p className="settings__card-description">
          {t("cadence_they_appear_chips_search_bar_ctrl")}
        </p>
        <ul className="settings__modes">
          {savedQuestions.length === 0 ? (
            <li className="settings__hint">{t("cadence_no_saved_questions")}</li>
          ) : null}
          {savedQuestions.map((question) => (
            <li key={question.id} className="settings__mode">
              <input
                className="settings__input"
                type="text"
                aria-label={t("cadence_saved_question")}
                maxLength={MAX_SAVED_QUESTION_CHARS}
                value={question.text}
                placeholder={t("cadence_e_g_how_can_i_optimize")}
                onChange={(event) =>
                  setSavedQuestions(
                    savedQuestions.map((item) =>
                      item.id === question.id
                        ? { ...item, text: event.target.value }
                        : item,
                    ),
                  )
                }
              />
              <button
                type="button"
                className="settings__button settings__button--small"
                aria-label={t("cadence_remove_saved_question")}
                onClick={() =>
                  setSavedQuestions(
                    savedQuestions.filter((item) => item.id !== question.id),
                  )
                }
              >
                {t("cadence_delete")}
              </button>
            </li>
          ))}
        </ul>
        <div className="settings__actions">
          <button
            type="button"
            className="settings__button"
            disabled={savedQuestions.length >= MAX_SAVED_QUESTIONS}
            onClick={() =>
              setSavedQuestions([
                ...savedQuestions,
                { id: `q-${Date.now()}-${savedQuestions.length}`, text: "" },
              ])
            }
          >
            {t("cadence_new_saved_question")}
          </button>
        </div>
      </article>
    </section>
  );
}
