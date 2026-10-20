import { useCallback, useEffect, useState } from "react";
import { saveSettings } from "../../adapters/ipc/settings";
import {
  captureClear,
  capturePending,
  capturedDataUrl,
  screenCapture,
  type CapturedScreen,
} from "../../adapters/ipc/screen";
import { subscribe } from "../../adapters/ipc/events";
import { useJournal } from "../../core/application/useJournal";
import { useSessionState } from "../../core/application/useSessionState";
import { useSettings } from "../../core/application/useSettings";
import type { JournalEntry } from "../../core/domain/journal";
import {
  DEFAULT_SAVED_QUESTIONS,
  MAX_SAVED_QUESTIONS,
  MAX_SAVED_QUESTION_CHARS,
  type SavedQuestion,
} from "../../core/domain/settings";
import { AskComposer } from "./AskComposer";
import { JournalEntryCard } from "./JournalEntryCard";
import { SavedQuestionsEditor } from "./SavedQuestionsEditor";

/**
 * Aba **Sessão**: o diário de perguntas (BR-052) e a feature **Busca**
 * (BR-053) — perguntar, print de tela e perguntas prontas.
 */
export function JournalView() {
  const { entries, loading, error, busy, query, setQuery, ask } = useJournal();
  const { settings, reload } = useSettings();
  const { state } = useSessionState();
  const [question, setQuestion] = useState("");
  const [attachment, setAttachment] = useState<CapturedScreen | null>(null);
  const [captureError, setCaptureError] = useState<string | null>(null);
  const [editingPresets, setEditingPresets] = useState(false);

  const savedQuestions = settings.saved_questions ?? DEFAULT_SAVED_QUESTIONS;

  // Recarrega as settings quando a aba abre (o editor pode ter salvado).
  useEffect(() => {
    void reload();
  }, [reload]);

  // O anexo é o **print pendente** do core (a prévia no canto é a mesma coisa):
  // ao abrir a aba e sempre que ele muda, o estado local acompanha.
  useEffect(() => {
    void capturePending().then((pending) => setAttachment(pending ?? null));
    let cancelled = false;
    let stop: (() => void) | undefined;
    void subscribe<null>("capture_changed", () => {
      void capturePending().then((pending) => {
        if (!cancelled) {
          setAttachment(pending ?? null);
        }
      });
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        stop = unlisten;
      }
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);

  const submit = useCallback(
    async (text: string, source: "search" | "preset" = "search") => {
      const pending = await capturePending();
      const images = pending
        ? [{ mime: pending.mime, data: pending.data }]
        : [];
      const outcome = await ask(text, images, source);
      if (outcome) {
        setQuestion("");
        setAttachment(null);
        if (images.length > 0) {
          await captureClear();
        }
      }
    },
    [ask],
  );

  // Imagem colada da área de transferência (data URL → base64).
  const pasteImage = useCallback((dataUrl: string) => {
    const match = /^data:([^;]+);base64,(.*)$/.exec(dataUrl);
    if (!match) {
      return;
    }
    const mime = match[1] ?? "";
    const data = match[2] ?? "";
    if (mime === "" || data === "") {
      return;
    }
    setCaptureError(null);
    setAttachment({ mime, data, width: 0, height: 0 });
  }, []);

  const capture = useCallback(async () => {
    setCaptureError(null);
    try {
      const captured = await screenCapture();
      if (!captured) {
        setCaptureError("Captura de tela indisponível — cole a imagem com Ctrl+V.");
      }
    } catch (caught) {
      setCaptureError(String(caught));
    }
  }, []);

  const savePreset = useCallback(async () => {
    const text = question.trim();
    if (text === "") {
      return;
    }
    if (text.length > MAX_SAVED_QUESTION_CHARS) {
      setCaptureError(
        `A pergunta tem no máximo ${MAX_SAVED_QUESTION_CHARS} caracteres.`,
      );
      return;
    }
    if (savedQuestions.length >= MAX_SAVED_QUESTIONS) {
      setCaptureError(`No máximo ${MAX_SAVED_QUESTIONS} perguntas prontas.`);
      return;
    }
    const next: SavedQuestion[] = [
      ...savedQuestions,
      { id: `q-${Date.now()}`, text },
    ];
    await saveSettings({ ...settings, saved_questions: next });
    await reload();
  }, [question, savedQuestions, settings, reload]);

  const persistPresets = useCallback(
    async (questions: SavedQuestion[]) => {
      await saveSettings({ ...settings, saved_questions: questions });
      await reload();
      setEditingPresets(false);
    },
    [settings, reload],
  );

  const retry = useCallback(
    (entry: JournalEntry) => {
      void submit(entry.question, entry.request_source === "preset" ? "preset" : "search");
    },
    [submit],
  );

  return (
    <div className="journal" data-interactive>
      <AskComposer
        value={question}
        onChange={setQuestion}
        savedQuestions={savedQuestions.map((item) => item.text)}
        onSubmit={(text, source) => {
          void submit(text, source);
        }}
        onCapture={() => void capture()}
        attachment={attachment ? capturedDataUrl(attachment) : null}
        onRemoveAttachment={() => {
          setAttachment(null);
          void captureClear();
        }}
        onPasteImage={pasteImage}
        onSavePreset={() => void savePreset()}
        onOpenPresets={() => setEditingPresets((previous) => !previous)}
        busy={busy}
        error={captureError}
      />

      {editingPresets && (
        <SavedQuestionsEditor
          questions={savedQuestions}
          onSave={(questions) => void persistPresets(questions)}
          onClose={() => setEditingPresets(false)}
        />
      )}

      <div className="journal__search">
        <input
          className="journal__search-input"
          value={query}
          placeholder="Buscar no diário…"
          aria-label="Buscar no diário"
          onChange={(event) => setQuery(event.target.value)}
        />
      </div>

      {error && <p className="overlay__error">{error}</p>}

      <div className="journal__list">
        {loading && entries.length === 0 && (
          <p className="chat__empty">Carregando…</p>
        )}

        {!loading && entries.length === 0 && (
          <p className="chat__empty">
            Nenhuma pergunta ainda. Use <strong>Perguntar</strong> para perguntar
            à IA.
          </p>
        )}

        {entries.map((entry) => (
          <JournalEntryCard
            key={entry.id}
            entry={entry}
            sessionStartedAtMs={state.started_at_ms}
            onRetry={retry}
          />
        ))}
      </div>
    </div>
  );
}
