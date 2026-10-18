import { useCallback, useEffect, useRef, useState } from "react";
import { sessionMessages } from "../../adapters/ipc/database";
import { subscribe } from "../../adapters/ipc/events";
import { llmInsight } from "../../adapters/ipc/llm";
import {
  toSuggestion,
  type InsightSuggestion,
} from "../domain/transcript";

/**
 * Sugestões da sessão, com o tempo de cada uma.
 *
 * Carrega o que já está gravado (`messages`) ao entrar na sessão, acrescenta as
 * novas (pedidas pelo painel ou pelo atalho global) e mantém tudo ordenado pela
 * linha temporal — é o que entra no painel ao vivo e no export Markdown.
 */
export function useInsight(sessionId: string | null): {
  suggestions: InsightSuggestion[];
  latest: InsightSuggestion | null;
  loading: boolean;
  error: string | null;
  request: () => Promise<void>;
} {
  const [suggestions, setSuggestions] = useState<InsightSuggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sessionRef = useRef<string | null>(null);

  const append = useCallback((suggestion: InsightSuggestion) => {
    setSuggestions((prev) =>
      prev.some(
        (item) =>
          suggestion.message_id !== null &&
          item.message_id !== null &&
          item.message_id === suggestion.message_id,
      )
        ? prev
        : [...prev, suggestion].sort((a, b) => a.at_ms - b.at_ms),
    );
  }, []);

  // Histórico da sessão atual.
  useEffect(() => {
    sessionRef.current = sessionId;
    if (!sessionId) {
      setSuggestions([]);
      return;
    }

    let cancelled = false;
    void sessionMessages(sessionId).then((loaded) => {
      if (cancelled || !loaded) {
        return;
      }
      setSuggestions(
        loaded
          .filter((message) => message.role === "assistant")
          .map(toSuggestion)
          .sort((a, b) => a.at_ms - b.at_ms),
      );
    });

    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  // Sugestões pedidas por atalho global (evento do core, já persistidas).
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<InsightSuggestion>("insight", (suggestion) => {
      if (suggestion.session_id !== sessionRef.current) {
        return;
      }
      append(suggestion);
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
  }, [append]);

  const request = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const suggestion = await llmInsight(sessionId);
      if (suggestion) {
        append(suggestion);
      }
    } catch (caught) {
      setError(String(caught));
    } finally {
      setLoading(false);
    }
  }, [append, sessionId]);

  return {
    suggestions,
    latest: suggestions.length > 0 ? suggestions[suggestions.length - 1]! : null,
    loading,
    error,
    request,
  };
}
