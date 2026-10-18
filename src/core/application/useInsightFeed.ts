import { useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import type { InsightSuggestion } from "../domain/transcript";

/** Assina o evento `insight` (gerado por atalho) e devolve a última sugestão. */
export function useInsightFeed(): InsightSuggestion | null {
  const [suggestion, setSuggestion] = useState<InsightSuggestion | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<InsightSuggestion>("insight", setSuggestion).then((stop) => {
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

  return suggestion;
}
