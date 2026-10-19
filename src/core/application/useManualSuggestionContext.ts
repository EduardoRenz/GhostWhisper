import { useCallback, useEffect, useRef, useState } from "react";
import {
  clearManualSuggestionContext,
  manualSuggestionContext,
} from "../../adapters/ipc/suggestions";
import { subscribe } from "../../adapters/ipc/events";
import type { ManualSuggestionContext } from "../domain/assistant";

interface UseManualSuggestionContextOptions {
  sessionId: string | null;
}

/**
 * Contexto manual vindo da Transcrição (BR-041).
 *
 * A janela de Sugestões é aberta pelo botão `[✦]` de uma mensagem; o contexto
 * fica no core e é buscado ao montar e a cada `panel_opened` (a janela pode já
 * estar aberta quando um novo clique chega). Trocar de sessão limpa o contexto
 * da sessão anterior.
 */
export function useManualSuggestionContext({
  sessionId,
}: UseManualSuggestionContextOptions): {
  context: ManualSuggestionContext | null;
  loading: boolean;
  error: string | null;
  clear: () => Promise<void>;
  refresh: () => Promise<void>;
} {
  const [context, setContext] = useState<ManualSuggestionContext | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sessionRef = useRef<string | null>(sessionId);
  const previousSession = useRef<string | null>(null);

  const refresh = useCallback(async () => {
    const current = sessionRef.current;
    if (!current) {
      setContext(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const loaded = await manualSuggestionContext(current);
      if (sessionRef.current === current) {
        setContext(loaded ?? null);
      }
    } catch (caught) {
      if (sessionRef.current === current) {
        setError(String(caught));
      }
    } finally {
      if (sessionRef.current === current) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    sessionRef.current = sessionId;
    const previous = previousSession.current;
    if (previous && previous !== sessionId) {
      void clearManualSuggestionContext(previous);
    }
    previousSession.current = sessionId;
    setContext(null);
    setError(null);
    void refresh();
  }, [sessionId, refresh]);

  // Um novo clique da Transcrição reusa `panel_open`; a janela relê o store.
  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    void subscribe<string>("panel_opened", (payload) => {
      if (payload === "suggestions") {
        void refresh();
      }
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
  }, [refresh]);

  const clear = useCallback(async () => {
    const current = sessionRef.current;
    if (!current) {
      setContext(null);
      return;
    }
    setError(null);
    try {
      await clearManualSuggestionContext(current);
      setContext(null);
    } catch (caught) {
      setError(String(caught));
    }
  }, []);

  return { context, loading, error, clear, refresh };
}
