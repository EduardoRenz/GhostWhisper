import { useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";

/** Canal da atividade de transcrição (espelha o evento do core). */
export type TranscriptActivityChannel = "microphone" | "system";

/** Evento `transcript_activity`: fala começou/terminou de ser processada. */
export interface TranscriptActivityEvent {
  session_id: string;
  channel: TranscriptActivityChannel;
  processing: boolean;
}

/** Estado por canal: `true` enquanto a fala daquele canal está em processamento. */
export interface TranscriptActivity {
  microphone: boolean;
  system: boolean;
}

const IDLE: TranscriptActivity = { microphone: false, system: false };

/**
 * Assina a atividade de transcrição do core.
 *
 * O sinal começa no primeiro bloco de fala e só apaga quando o passe final do
 * turno termina — por isso não pisca entre prévias. Eventos de outra sessão são
 * ignorados; trocar de sessão zera o estado.
 */
export function useTranscriptActivity(
  sessionId: string | null,
  enabled = true,
): TranscriptActivity {
  const [activity, setActivity] = useState<TranscriptActivity>(IDLE);

  useEffect(() => {
    setActivity(IDLE);
    if (!sessionId || !enabled) {
      return;
    }

    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void subscribe<TranscriptActivityEvent>(
      "transcript_activity",
      (incoming) => {
        if (cancelled || !incoming || incoming.session_id !== sessionId) {
          return;
        }
        if (
          incoming.channel !== "microphone" &&
          incoming.channel !== "system"
        ) {
          return;
        }
        setActivity((previous) =>
          previous[incoming.channel] === incoming.processing
            ? previous
            : { ...previous, [incoming.channel]: incoming.processing },
        );
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
  }, [sessionId, enabled]);

  return activity;
}
