import { useCallback, useEffect, useState } from "react";
import {
  deleteSession,
  listSessions,
  renameSession,
  sessionTimeline,
} from "../../adapters/ipc/database";
import type {
  SessionSummary,
  TimelineEntry,
} from "../domain/transcript";

/** Quantas sessões o histórico carrega por vez. */
export const HISTORY_LIMIT = 50;

/**
 * Lista de sessões do histórico + ações (renomear/excluir).
 *
 * Recarrega quando a sessão ativa muda, para a sessão que acabou de começar
 * aparecer no topo.
 */
export function useSessionHistory(activeSessionId: string | null): {
  sessions: SessionSummary[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  rename: (id: string, title: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
} {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const loaded = await listSessions(HISTORY_LIMIT);
      if (loaded) {
        setSessions(loaded);
      }
    } catch (caught) {
      setError(String(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh, activeSessionId]);

  const rename = useCallback(
    async (id: string, title: string) => {
      setError(null);
      try {
        await renameSession(id, title);
      } catch (caught) {
        setError(String(caught));
        return;
      }
      await refresh();
    },
    [refresh],
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
      await refresh();
    },
    [refresh],
  );

  return { sessions, loading, error, refresh, rename, remove };
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
