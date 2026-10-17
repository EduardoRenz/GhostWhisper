import { useCallback, useState } from "react";
import { llmInsight } from "../../adapters/ipc/llm";

/** Pede uma sugestão contextual ao provider configurado. */
export function useInsight(sessionId: string | null): {
  insight: string | null;
  loading: boolean;
  error: string | null;
  request: () => Promise<void>;
  clear: () => void;
} {
  const [insight, setInsight] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const request = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await llmInsight(sessionId);
      setInsight(result ?? null);
    } catch (caught) {
      setError(String(caught));
    } finally {
      setLoading(false);
    }
  }, [sessionId]);

  const clear = useCallback(() => {
    setInsight(null);
    setError(null);
  }, []);

  return { insight, loading, error, request, clear };
}
