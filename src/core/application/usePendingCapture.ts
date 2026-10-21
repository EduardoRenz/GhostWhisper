/**
 * Print **pendente** do core (BR-055), o mesmo que a prévia no canto mostra.
 *
 * A captura só **anexa**: o print fica guardado no core até o envio (ou o
 * remover). Assim a aba Sessão e a barra de pesquisa leem o mesmo estado.
 */
import { useCallback, useEffect, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import {
  captureClear,
  capturePending,
  screenCapture,
  type CapturedScreen,
} from "../../adapters/ipc/screen";

export interface PendingCapture {
  /** Print anexado (ainda não enviado). */
  attachment: CapturedScreen | null;
  /** Uma captura está em curso. */
  capturing: boolean;
  error: string | null;
  /** Captura e **anexa** (não envia nada). */
  capture: () => Promise<void>;
  /** Descarta o print pendente (fecha a prévia). */
  clear: () => Promise<void>;
}

/** Acompanha o print pendente do core. */
export function usePendingCapture(): PendingCapture {
  const [attachment, setAttachment] = useState<CapturedScreen | null>(null);
  const [capturing, setCapturing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // O print pode ter sido capturado em outra janela: acompanha o core.
  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    const sync = () => {
      void capturePending().then((pending) => {
        if (!cancelled) {
          setAttachment(pending ?? null);
        }
      });
    };
    sync();
    void subscribe<null>("capture_changed", sync).then((unlisten) => {
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
  }, []);

  const capture = useCallback(async () => {
    setCapturing(true);
    setError(null);
    try {
      const shot = await screenCapture();
      if (!shot) {
        setError("Captura de tela indisponível — cole a imagem com Ctrl+V.");
      }
    } catch (caught) {
      setError(String(caught));
    } finally {
      setCapturing(false);
    }
  }, []);

  const clear = useCallback(async () => {
    setAttachment(null);
    await captureClear();
  }, []);

  return { attachment, capturing, error, capture, clear };
}
