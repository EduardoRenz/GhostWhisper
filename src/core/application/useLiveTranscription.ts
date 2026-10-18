import { useEffect, useRef, useState } from "react";
import { sessionTranscriptions } from "../../adapters/ipc/database";
import { subscribe } from "../../adapters/ipc/events";
import {
  mergeHistory,
  mergeSegment,
  type TranscriptSegment,
} from "../domain/transcript";

/**
 * Mantém a linha temporal da sessão em tela.
 *
 * Ao entrar numa sessão (inclusive ao **retomar** ou **continuar**, BR-019) o
 * histórico persistido é recarregado do banco e mesclado com os eventos ao vivo
 * — as mensagens não são mais descartadas quando a captura para e volta.
 */
export function useLiveTranscription(sessionId: string | null): {
  segments: TranscriptSegment[];
} {
  const [segments, setSegments] = useState<TranscriptSegment[]>([]);
  /** Sessão a que a lista em tela pertence. */
  const sessionRef = useRef<string | null>(null);

  // Troca de sessão: descarta o que era de outra sessão e recarrega o
  // histórico persistido dela (BR-019).
  useEffect(() => {
    sessionRef.current = sessionId;
    if (!sessionId) {
      setSegments([]);
      return;
    }

    setSegments((prev) => prev.filter((item) => item.session_id === sessionId));

    let cancelled = false;
    void sessionTranscriptions(sessionId).then((loaded) => {
      if (cancelled || !loaded) {
        return;
      }
      // Os eventos ao vivo podem ter chegado antes da resposta: mescla em vez
      // de sobrepor.
      setSegments((prev) =>
        mergeHistory(
          loaded,
          prev.filter((item) => item.session_id === sessionId),
        ),
      );
    });

    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  // Eventos ao vivo (prévia e final compartilham o id).
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<TranscriptSegment>("transcript_segment", (segment) => {
      // Um evento atrasado de outra sessão é ignorado (não contamina a lista).
      if (segment.session_id !== sessionRef.current) {
        return;
      }
      setSegments((prev) => mergeSegment(prev, segment));
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

  return { segments };
}
