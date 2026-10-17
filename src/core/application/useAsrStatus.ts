import { useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import type { AsrPhase, AsrStatusEvent } from "../domain/asr";

/**
 * Assina `asr_status` e devolve o último status do motor de transcrição.
 *
 * Trocar o modelo do Whisper pode baixar centenas de MB: sem este retorno o
 * usuário só via uma janela "travada" sem explicação.
 */
export function useAsrStatus(): AsrStatusEvent | null {
  const [status, setStatus] = useState<AsrStatusEvent | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void subscribe<AsrStatusEvent>("asr_status", (payload) => {
      setStatus({
        phase: payload.phase as AsrPhase,
        model: payload.model,
        message: payload.message,
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

  return status;
}