import { useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";

/** Assina o evento `insight` (gerado por atalho) e devolve o último texto. */
export function useInsightFeed(): string | null {
  const [insight, setInsight] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<string>("insight", setInsight).then((stop) => {
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

  return insight;
}
