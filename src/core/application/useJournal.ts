/**
 * Hook do diário de perguntas (aba "Sessão", BR-052).
 *
 * Carrega o diário da **sessão ativa**, aplica o **streaming** da resposta
 * (evento `journal`) e envia perguntas. Encerrar a sessão (ou abrir outra)
 * zera a aba: cada sessão tem o seu diário.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  /** Entradas do diário, **da mais recente para a mais antiga**. */
  entries: JournalEntry[];
  /** Total de entradas da sessão (sem a busca). */
  count: number;
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
 * `activeSessionId` é a sessão em andamento no core: quando ela muda (encerrou,
 * continuou ou abriu outra), a lista é **descartada** e recarregada — a aba
 * nunca mostra o diário da sessão anterior.
 */
export function useJournal(activeSessionId: string | null = null): UseJournal {
  const [entries, setEntries] = useState<JournalEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [query, setQuery] = useState("");
  const [searchResults, setSearchResults] = useState<JournalEntry[] | null>(null);
  /** Ids já na lista (para saber, no evento, se a entrada é nova). */
  const knownIds = useRef<Set<string>>(new Set());

  useEffect(() => {
    knownIds.current = new Set(entries.map((entry) => entry.id));
  }, [entries]);

  /**
   * Carrega o diário da sessão.
   *
   * `merge` preserva o que já está na tela: o reload que roda no fim da
   * resposta existe só para trazer origem/modelo/instantes, **nunca** para
   * apagar a resposta que acabou de chegar (a lista pode vir vazia).
   */
  const load = useCallback(async (merge = false) => {
    setLoading(true);
    try {
      const listed = await journalList();
      const fresh = listed ?? [];
      setEntries((previous) => (merge ? mergeEntries(previous, fresh) : fresh));
      setError(null);
    } catch (caught) {
      setError(String(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  // A aba é da **sessão ativa**: encerrar some com o diário; uma sessão nova
  // começa vazia. Roda também na montagem.
  useEffect(() => {
    setEntries([]);
    setSearchResults(null);
    void load();
  }, [activeSessionId, load]);

  // Streaming da resposta: a entrada ganha o texto acumulado a cada evento.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<JournalUpdate>(JOURNAL_EVENT, (update) => {
      if (cancelled) {
        return;
      }
      if (!knownIds.current.has(update.entry_id)) {
        if (update.phase !== "pending") {
          // Entrada que não está na lista (ex.: pergunta feita fora da aba):
          // recarrega do banco em vez de inventar uma linha.
          void load(true);
          return;
        }
        // Entrada nova (a pergunta acabou de ser registrada).
        knownIds.current.add(update.entry_id);
        setEntries((previous) => [
          ...previous,
          {
            id: update.entry_id,
            session_id: activeSessionId ?? "",
            question: update.text,
            answer: null,
            asked_at_ms: 0,
            answered_at_ms: null,
            created_at_ms: Date.now(),
            status: "pending",
            request_source: "search",
            provider: null,
            model: null,
            error: null,
          },
        ]);
        return;
      }
      setEntries((previous) =>
        previous.map((entry) => {
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
        }),
      );
      // Fim da resposta: recarrega a entrada do banco para trazer o que o
      // streaming não carrega (origem, modelo, instantes) — sem apagar nada.
      if (update.phase === "answered" || update.phase === "error") {
        void load(true);
      }
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
  }, [activeSessionId, load]);

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

  // A lista mostra a interação **mais recente primeiro** (a sessão é um diário
  // que cresce para cima); o core continua devolvendo ordem cronológica.
  const visible = useMemo(
    () => [...(searchResults ?? entries)].reverse(),
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
          // Com print, a origem é `screenshot` (é ela que carrega a miniatura
          // e mostra "Screenshot + Texto"); sem print vale a origem pedida.
          source: images.length > 0 ? "screenshot" : source,
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
    count: entries.length,
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

/**
 * Mescla a lista do banco na que está na tela.
 *
 * O que veio do banco manda (origem, modelo, instantes), mas uma entrada que
 * ainda não foi persistida fica, e a resposta em streaming nunca regride para
 * "sem resposta" (o banco pode estar um passo atrás).
 */
function mergeEntries(
  current: JournalEntry[],
  fresh: JournalEntry[],
): JournalEntry[] {
  if (fresh.length === 0) {
    // Nada no banco (ex.: sessão ainda não resolvida): não apague a tela.
    return current;
  }
  const saved = new Map(fresh.map((entry) => [entry.id, entry]));
  const merged = current.map((entry) => {
    const stored = saved.get(entry.id);
    if (!stored) {
      return entry;
    }
    return { ...stored, answer: stored.answer ?? entry.answer };
  });
  for (const entry of fresh) {
    if (!current.some((item) => item.id === entry.id)) {
      merged.push(entry);
    }
  }
  return merged;
}
