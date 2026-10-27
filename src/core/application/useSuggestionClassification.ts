import { useCallback, useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import {
  suggestionClassificationLast,
  suggestionClassifyNow,
} from "../../adapters/ipc/suggestions";
import type {
  SuggestionClassification,
  SuggestionClassificationEvent,
  SuggestionClassifierState,
} from "../domain/assistant";

export interface SuggestionClassificationView {
  state: SuggestionClassifierState;
  outcome: SuggestionClassification | null;
  error: string | null;
  generating: boolean;
}

const EMPTY: SuggestionClassificationView = {
  state: "listening",
  outcome: null,
  error: null,
  generating: false,
};

/**
 * Assina a classificação mais recente do core (BR-095).
 *
 * Hidrata ao abrir/trocar de sessão, atualiza por evento e permite classificar
 * o bloco atual sem gerar sugestão. O core é a fonte da verdade; a UI não
 * recalcula cue/confiança.
 */
export function useSuggestionClassification(sessionId: string | null) {
  const [view, setView] = useState<SuggestionClassificationView>(EMPTY);

  useEffect(() => {
    if (!sessionId) {
      setView({ ...EMPTY, state: "no_session" });
      return;
    }

    let cancelled = false;
    let unlisten: (() => void) | undefined;
    setView((previous) => ({
      ...previous,
      state: "listening",
      outcome: null,
      error: null,
    }));

    void suggestionClassificationLast(sessionId).then((last) => {
      if (cancelled) return;
      if (last) {
        setView({
          state: "ready",
          outcome: last,
          error: null,
          generating: false,
        });
      }
    });

    void subscribe<SuggestionClassificationEvent>(
      "suggestion_classification",
      (incoming) => {
        if (cancelled || !incoming) return;
        if (incoming.session_id && incoming.session_id !== sessionId) return;
        setView({
          state: incoming.state,
          outcome: incoming.outcome,
          error: incoming.error,
          generating: incoming.generating,
        });
      },
    ).then((stop) => {
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
  }, [sessionId]);

  const classifyNow = useCallback(async () => {
    if (!sessionId) return;
    setView((previous) => ({
      ...previous,
      state: "analyzing",
      error: null,
    }));
    try {
      const outcome = await suggestionClassifyNow(sessionId);
      if (outcome) {
        setView({
          state: "ready",
          outcome,
          error: null,
          generating: false,
        });
      }
    } catch (error) {
      setView((previous) => ({
        ...previous,
        state: "error",
        error: error instanceof Error ? error.message : String(error),
      }));
    }
  }, [sessionId]);

  return { ...view, classifyNow };
}
