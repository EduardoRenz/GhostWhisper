import { useCallback, useEffect, useRef, useState } from "react";
import {
  deleteSession,
  listSessions,
  renameSession,
  searchSessions,
  sessionTimeline,
} from "../../adapters/ipc/database";
import { subscribe } from "../../adapters/ipc/events";
import type {
  SessionSummary,
  TimelineEntry,
} from "../domain/transcript";

/** Quantas sessões o histórico carrega por vez. */
export const HISTORY_LIMIT = 50;

/** Espera antes de consultar o banco enquanto o usuário digita (BR-103). */
export const HISTORY_SEARCH_DEBOUNCE_MS = 250;

/** Evento do core quando uma sessão muda (resumo/título em background). */
const SESSION_UPDATED_EVENT = "session_updated";

interface SessionUpdatedEvent {
  session_id: string;
  subject: string | null;
}

interface LoadOptions {
  /** Não mexe no indicador de carregamento (atualização em background). */
  silent?: boolean;
}

/**
 * Lista de sessões do histórico + ações (renomear/excluir).
 *
 * Recarrega quando a sessão ativa muda, para a sessão que acabou de começar
 * aparecer no topo, e quando a pesquisa muda (busca no banco, com debounce).
 * O resumo/título gerado no encerramento chega pelo evento `session_updated`.
 */
export function useSessionHistory(
  activeSessionId: string | null,
  query = "",
): {
  sessions: SessionSummary[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  rename: (id: string, title: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  removeMany: (ids: string[]) => Promise<{ deleted: number; failed: number }>;
} {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [debouncedQuery, setDebouncedQuery] = useState(() => query.trim());
  /** Descarta respostas de uma pesquisa que já foi substituída. */
  const requestId = useRef(0);

  // Debounce só quando a consulta muda de fato (a primeira carga é imediata;
  // limpar a busca volta à lista na hora).
  useEffect(() => {
    const trimmed = query.trim();
    if (trimmed === debouncedQuery) {
      return;
    }
    if (trimmed === "") {
      setDebouncedQuery("");
      return;
    }
    const timer = setTimeout(
      () => setDebouncedQuery(trimmed),
      HISTORY_SEARCH_DEBOUNCE_MS,
    );
    return () => clearTimeout(timer);
  }, [query, debouncedQuery]);

  const load = useCallback(
    async (options: LoadOptions = {}) => {
      const current = ++requestId.current;
      if (!options.silent) {
        setLoading(true);
      }
      setError(null);
      try {
        const loaded = debouncedQuery
          ? await searchSessions(debouncedQuery, HISTORY_LIMIT)
          : await listSessions(HISTORY_LIMIT);
        if (current === requestId.current && loaded) {
          setSessions(loaded);
        }
      } catch (caught) {
        if (current === requestId.current) {
          setError(String(caught));
        }
      } finally {
        if (current === requestId.current && !options.silent) {
          setLoading(false);
        }
      }
    },
    [debouncedQuery],
  );

  useEffect(() => {
    void load();
  }, [load, activeSessionId]);

  // Resumo/título gerado no encerramento (BR-104): recarrega sem piscar.
  useEffect(() => {
    let stop: (() => void) | null = null;
    let cancelled = false;
    void subscribe<SessionUpdatedEvent>(SESSION_UPDATED_EVENT, () => {
      void load({ silent: true });
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [load]);

  const rename = useCallback(
    async (id: string, title: string) => {
      setError(null);
      try {
        await renameSession(id, title);
      } catch (caught) {
        setError(String(caught));
        return;
      }
      await load();
    },
    [load],
  );

  const remove = useCallback(
    async (id: string) => {
      setError(null);
      try {
        await deleteSession(id);
      } catch (caught) {
        setError(String(caught));
        return;
      }
      await load();
    },
    [load],
  );

  /**
   * Exclui várias sessões em sequência (BR-103).
   *
   * Sequencial de propósito: cada exclusão pode parar a sessão ativa e
   * descartar a gravação. O recarregamento é único, no fim.
   */
  const removeMany = useCallback(
    async (ids: string[]) => {
      setError(null);
      let deleted = 0;
      let failed = 0;
      for (const id of ids) {
        try {
          await deleteSession(id);
          deleted += 1;
        } catch {
          failed += 1;
        }
      }
      await load();
      return { deleted, failed };
    },
    [load],
  );

  return { sessions, loading, error, refresh: load, rename, remove, removeMany };
}

/** Linha temporal de uma sessão escolhida (falas + sugestões). */
export function useSessionTimeline(sessionId: string | null): {
  entries: TimelineEntry[];
  loading: boolean;
  error: string | null;
} {
  const [entries, setEntries] = useState<TimelineEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!sessionId) {
      setEntries([]);
      return;
    }

    let cancelled = false;
    setLoading(true);
    setError(null);
    void sessionTimeline(sessionId)
      .then((loaded) => {
        if (!cancelled && loaded) {
          setEntries(loaded);
        }
      })
      .catch((caught) => {
        if (!cancelled) {
          setError(String(caught));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  return { entries, loading, error };
}
