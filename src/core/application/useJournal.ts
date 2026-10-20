/**
 * Hook do diário de perguntas (aba "Sessão", BR-052).
 *
 * Carrega o diário da sessão, aplica o **streaming** da resposta (evento
 * `journal`) e envia perguntas.
 */
import { useCallback, useEffect, useMemo, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import {
  journalAsk,
  journalCancel,
  journalList,
  journalSearch,
  type AttachedImage,
  type JournalAskOutcome,
} from "../../adapters/ipc/journal";
import type {
  JournalEntry,
  JournalRequestSource,
  JournalUpdate,
} from "../domain/journal";

/** Evento do core com o progresso de uma entrada. */
const JOURNAL_EVENT = "journal";

/** Debounce da busca (ms) — paridade com a busca de sessões do Perssua. */
const SEARCH_DEBOUNCE_MS = 180;

export interface UseJournal {
  /** Entradas do diário (ou da busca, quando há). */
  entries: JournalEntry[];
  loading: boolean;
  error: string | null;
  /** `true` enquanto uma pergunta está em curso. */
  busy: boolean;
  /** Termo da busca ("" = sem busca). */
  query: string;
  setQuery: (query: string) => void;
  /** Refaz a busca com o termo atual. */
  refresh: () => void;
  /** Pergunta à IA. */
  ask: (
    question: string,
    images?: AttachedImage[],
    source?: JournalRequestSource,
  ) => Promise<JournalAskOutcome | undefined>;
  /** Cancela uma pergunta em curso. */
  cancel: (entryId: string) => Promise<void>;
}

/**
 * Diário da sessão.
 *
 * A sessão é resolvida pelo core (a ativa ou a mais recente); a UI só
 * precisa da lista.
 */
export function useJournal(): UseJournal {
  const [entries, setEntries] = useState<JournalEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [query, setQuery] = useState("");
  const [searchResults, setSearchResults] = useState<JournalEntry[] | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const listed = await journalList();
      setEntries(listed ?? []);
      setError(null);
    } catch (caught) {
      setError(String(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // Streaming da resposta: a entrada ganha o texto acumulado a cada evento.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<JournalUpdate>(JOURNAL_EVENT, (update) => {
      if (cancelled) {
        return;
      }
      setEntries((previous) => {
        const known = previous.some((entry) => entry.id === update.entry_id);
        if (!known) {
          // Entrada nova (a pergunta acabou de ser registrada).
          return [
            ...previous,
            {
              id: update.entry_id,
              session_id: "",
              question: update.phase === "pending" ? update.text : "",
              answer: update.phase === "pending" ? null : update.text,
              asked_at_ms: 0,
              answered_at_ms: null,
              status:
                update.phase === "answered"
                  ? "answered"
                  : update.phase === "error"
                    ? "error"
                    : "pending",
              request_source: "search",
              provider: null,
              model: null,
              error: update.error,
            },
          ];
        }
        return previous.map((entry) => {
          if (entry.id !== update.entry_id) {
            return entry;
          }
          return {
            ...entry,
            answer: update.text,
            status:
              update.phase === "answered"
                ? "answered"
                : update.phase === "error"
                  ? "error"
                  : entry.status === "answered"
                    ? "answered"
                    : "pending",
            error: update.error ?? entry.error,
          };
        });
      });
    }).then((stop) => {
      if (cancelled) {
        stop();
      } else {
        unlisten = stop;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Busca com debounce (o core escapa a query — ISS-042).
  useEffect(() => {
    const term = query.trim();
    if (term === "") {
      setSearchResults(null);
      return;
    }
    const timer = window.setTimeout(() => {
      void journalSearch(null, term).then((found) => {
        setSearchResults(found ?? []);
      });
    }, SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [query]);

  const visible = useMemo(
    () => (searchResults ?? entries),
    [searchResults, entries],
  );

  const ask = useCallback(
    async (
      question: string,
      images: AttachedImage[] = [],
      source: JournalRequestSource = "search",
    ) => {
      setBusy(true);
      setError(null);
      try {
        const outcome = await journalAsk({
          question,
          images,
          source: images.length > 0 && source === "search" ? "screenshot" : source,
        });
        await load();
        return outcome;
      } catch (caught) {
        setError(String(caught));
        return undefined;
      } finally {
        setBusy(false);
      }
    },
    [load],
  );

  const cancel = useCallback(async (entryId: string) => {
    await journalCancel(entryId);
  }, []);

  const refresh = useCallback(() => {
    void load();
  }, [load]);

  return {
    entries: visible,
    loading,
    error,
    busy,
    query,
    setQuery,
    refresh,
    ask,
    cancel,
  };
}
