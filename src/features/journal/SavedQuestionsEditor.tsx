import { useState } from "react";
import {
  MAX_SAVED_QUESTIONS,
  MAX_SAVED_QUESTION_CHARS,
  type SavedQuestion,
} from "../../core/domain/settings";

interface SavedQuestionsEditorProps {
  questions: SavedQuestion[];
  /** Salva a lista inteira (nas settings). */
  onSave: (questions: SavedQuestion[]) => void;
  /** Fecha o editor. */
  onClose: () => void;
}

/**
 * Editor das **perguntas prontas** (BR-054).
 *
 * Limites: 20 perguntas × 200 caracteres. O que exceder é **recusado com
 * aviso** — nunca truncado em silêncio.
 */
export function SavedQuestionsEditor({
  questions,
  onSave,
  onClose,
}: SavedQuestionsEditorProps) {
  const [drafts, setDrafts] = useState<SavedQuestion[]>(questions);
  const [error, setError] = useState<string | null>(null);

  const update = (id: string, text: string) => {
    setDrafts((previous) =>
      previous.map((question) =>
        question.id === id ? { ...question, text } : question,
      ),
    );
  };

  const remove = (id: string) => {
    setDrafts((previous) => previous.filter((question) => question.id !== id));
  };

  const add = () => {
    if (drafts.length >= MAX_SAVED_QUESTIONS) {
      setError(`No máximo ${MAX_SAVED_QUESTIONS} perguntas.`);
      return;
    }
    setError(null);
    setDrafts((previous) => [
      ...previous,
      { id: `q-${Date.now()}-${previous.length}`, text: "" },
    ]);
  };

  const save = () => {
    const cleaned = drafts
      .map((question) => ({ ...question, text: question.text.trim() }))
      .filter((question) => question.text !== "");
    if (cleaned.length > MAX_SAVED_QUESTIONS) {
      setError(`No máximo ${MAX_SAVED_QUESTIONS} perguntas.`);
      return;
    }
    const tooLong = cleaned.find(
      (question) => question.text.length > MAX_SAVED_QUESTION_CHARS,
    );
    if (tooLong) {
      setError(
        `Cada pergunta tem no máximo ${MAX_SAVED_QUESTION_CHARS} caracteres.`,
      );
      return;
    }
    onSave(cleaned);
  };

  return (
    <div className="presets" data-interactive>
      <div className="presets__head">
        <span className="presets__title">Perguntas prontas</span>
        <span className="presets__count">
          {drafts.length}/{MAX_SAVED_QUESTIONS}
        </span>
      </div>

      <div className="presets__list">
        {drafts.map((question) => (
          <div key={question.id} className="presets__row">
            <input
              className="presets__input"
              value={question.text}
              maxLength={MAX_SAVED_QUESTION_CHARS}
              aria-label={`Pergunta pronta ${question.id}`}
              placeholder="Escreva a pergunta…"
              onChange={(event) => update(question.id, event.target.value)}
            />
            <button
              type="button"
              className="presets__remove"
              title="Remover"
              aria-label={`Remover ${question.text || "pergunta"}`}
              onClick={() => remove(question.id)}
            >
              ×
            </button>
          </div>
        ))}
      </div>

      {error && <p className="presets__error">{error}</p>}

      <div className="presets__actions">
        <button type="button" className="overlay__button" onClick={add}>
          Adicionar
        </button>
        <button
          type="button"
          className="overlay__button overlay__button--start"
          onClick={save}
        >
          Salvar
        </button>
        <button type="button" className="overlay__button" onClick={onClose}>
          Fechar
        </button>
      </div>
    </div>
  );
}
