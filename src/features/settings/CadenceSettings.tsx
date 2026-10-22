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

interface CadenceSettingsProps {
  settings: Settings;
  update: <K extends keyof Settings>(field: K, value: Settings[K]) => void;
}

/** Cadências automáticas + perguntas prontas. */
export function CadenceSettings({ settings, update }: CadenceSettingsProps) {
  const savedQuestions = settings.saved_questions ?? DEFAULT_SAVED_QUESTIONS;
  const setSavedQuestions = (questions: SavedQuestion[]) =>
    update("saved_questions", questions);

  return (
    <section className="settings__section">
      <header className="settings__section-header">
        <h2 className="settings__section-title">
          Cadências &amp; Perguntas Prontas
        </h2>
        <span className="settings__section-note">
          Atualizações automáticas e chips da busca
        </span>
      </header>

      <div className="settings__grid">
        <article className="settings__card settings__card--hud">
          <header className="settings__card-header">
            <h3 className="settings__card-title">Resumo automático</h3>
            <label className="settings__toggle">
              <input
                type="checkbox"
                aria-label="Resumo automático"
                checked={settings.summary_auto_refresh}
                onChange={(event) =>
                  update("summary_auto_refresh", event.target.checked)
                }
              />
              <span className="settings__toggle-track" aria-hidden="true" />
            </label>
          </header>
          <p className="settings__card-description">
            Atualiza o resumo vivo da sessão enquanto a janela está aberta.
          </p>
          <label className="settings__field">
            <span>Intervalo do resumo</span>
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
            <h3 className="settings__card-title">Sugestões automáticas</h3>
            <label className="settings__toggle">
              <input
                type="checkbox"
                aria-label="Sugestões automáticas"
                checked={settings.suggestion_auto_refresh}
                onChange={(event) =>
                  update("suggestion_auto_refresh", event.target.checked)
                }
              />
              <span className="settings__toggle-track" aria-hidden="true" />
            </label>
          </header>
          <p className="settings__card-description">
            Gera uma nova sugestão a cada intervalo, sem esperar o clique.
          </p>
          <label className="settings__field">
            <span>Intervalo das sugestões</span>
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
          <h3 className="settings__card-title">Perguntas prontas</h3>
          <span className="settings__badge">
            {savedQuestions.length}/{MAX_SAVED_QUESTIONS}
          </span>
        </header>
        <p className="settings__card-description">
          Elas aparecem como chips na barra de pesquisa (Ctrl K).
        </p>
        <ul className="settings__modes">
          {savedQuestions.length === 0 ? (
            <li className="settings__hint">Nenhuma pergunta pronta.</li>
          ) : null}
          {savedQuestions.map((question) => (
            <li key={question.id} className="settings__mode">
              <input
                className="settings__input"
                type="text"
                aria-label="Pergunta pronta"
                maxLength={MAX_SAVED_QUESTION_CHARS}
                value={question.text}
                placeholder="Ex.: Como otimizar esta consulta?"
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
                aria-label="Remover pergunta pronta"
                onClick={() =>
                  setSavedQuestions(
                    savedQuestions.filter((item) => item.id !== question.id),
                  )
                }
              >
                Excluir
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
            Nova pergunta pronta
          </button>
        </div>
      </article>
    </section>
  );
}
