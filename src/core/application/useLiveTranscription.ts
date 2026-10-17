import { useEffect, useRef, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import type { TranscriptSegment } from "../domain/transcript";

/** Máximo de segmentos mantidos em tela (evita re-render/reflow excessivo). */
export const MAX_SEGMENTS = 60;

/**
 * Assina `transcript_segment` e mantém a lista viva (limitada) em memória.
 * Segmentos de uma nova sessão reiniciam a lista.
 */
export function useLiveTranscription(): {
  segments: TranscriptSegment[];
  clear: () => void;
} {
  const [segments, setSegments] = useState<TranscriptSegment[]>([]);
  const currentSession = useRef<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<TranscriptSegment>("transcript_segment", (segment) => {
      setSegments((prev) => {
        const sessionChanged =
          currentSession.current !== null &&
          currentSession.current !== segment.session_id;
        currentSession.current = segment.session_id;
        const base = sessionChanged ? [] : prev;
        // Preview e final compartilham o mesmo id: substitui em vez de duplicar.
        const index = base.findIndex((item) => item.id === segment.id);
        if (index === -1) {
          return [...base, segment].slice(-MAX_SEGMENTS);
        }
        const next = [...base];
        next[index] = segment;
        return next;
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

  const clear = () => {
    currentSession.current = null;
    setSegments([]);
  };

  return { segments, clear };
}
