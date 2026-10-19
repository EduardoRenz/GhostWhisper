import { useCallback, useEffect, useRef, useState } from "react";
import { dwellContext, dwellResult } from "../../adapters/ipc/dwell";
import { subscribe } from "../../adapters/ipc/events";
import {
  sampleDwellCursor,
  type DwellWindowGeometry,
} from "../../adapters/ipc/cursor";
import { isTauriRuntime } from "../../adapters/ipc/client";
import { DEFAULT_DWELL_MS } from "../../core/domain/dwell";
import { useDwellTimer } from "../../core/application/useDwellTimer";
import { DwellPopover } from "../../components/dwell/DwellPopover";

interface DwellContextPayload extends DwellWindowGeometry {
  dwell_ms: number;
  token: number;
}

interface DwellTriggerPayload {
  entered: boolean;
}

/**
 * Janela de confirmação do drill-hover (rota `dwell`, BR-036).
 *
 * - Recebe a duração do core (`dwell_context`) e liga o timer do "Sim".
 * - Avisa o overlay quando o cursor entra/sai (grace entre janelas).
 * - Confirma/cancela via `dwell_result`; o core esconde e devolve ao overlay.
 */
export function DwellPopoverWindow() {
  const [dwellMs, setDwellMs] = useState(DEFAULT_DWELL_MS);
  const [token, setToken] = useState(0);
  const [busy, setBusy] = useState(false);
  /** `true` enquanto a janela está aberta (inicia o polling do cursor). */
  const [open, setOpen] = useState(false);
  /** Geometria enviada pelo core no `dwell_context` (posição/tamanho reais). */
  const [geometry, setGeometry] = useState<DwellWindowGeometry | null>(null);
  const submitted = useRef(false);
  /** Cursor está sobre a janela de confirmação (pausa o grace). */
  const overRoot = useRef(false);
  /** Cursor está sobre o gatilho no overlay (pausa o grace). */
  const overTrigger = useRef(true);
  /** Cursor está sobre o "Sim" (evita reiniciar o dwell a cada amostra). */
  const overConfirm = useRef(false);
  /** Timer de cancelamento (só corre quando o cursor está fora das duas áreas). */
  const graceTimer = useRef<number | null>(null);

  const finish = useCallback((confirmed: boolean) => {
    if (submitted.current) {
      return;
    }
    submitted.current = true;
    setBusy(true);
    void dwellResult(confirmed).catch(() => {
      submitted.current = false;
      setBusy(false);
    });
  }, []);

  const timer = useDwellTimer(dwellMs, () => finish(true));
  const stop = timer.stop;
  const timerRef = useRef(timer);
  timerRef.current = timer;

  const clearGrace = useCallback(() => {
    if (graceTimer.current !== null) {
      window.clearTimeout(graceTimer.current);
      graceTimer.current = null;
    }
  }, []);

  const scheduleGrace = useCallback(() => {
    clearGrace();
    graceTimer.current = window.setTimeout(() => {
      graceTimer.current = null;
      finish(false);
    }, dwellMs);
  }, [clearGrace, dwellMs, finish]);

  /** Inicia o grace só se ainda não houver um correndo. */
  const ensureGrace = useCallback(() => {
    if (graceTimer.current === null) {
      scheduleGrace();
    }
  }, [scheduleGrace]);

  const reset = useCallback(() => {
    stop();
    clearGrace();
    overRoot.current = false;
    overTrigger.current = true;
    overConfirm.current = false;
    submitted.current = false;
    setBusy(false);
  }, [stop, clearGrace]);

  useEffect(() => () => clearGrace(), [clearGrace]);

  useEffect(() => {
    void dwellContext()
      .then((ms) => {
        if (typeof ms === "number") {
          setDwellMs(ms);
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    let stopContext: (() => void) | undefined;
    void subscribe<DwellContextPayload>("dwell_context", (payload) => {
      setDwellMs(payload.dwell_ms);
      setToken(payload.token);
      setGeometry({
        x: payload.x,
        y: payload.y,
        width: payload.width,
        height: payload.height,
        scale: payload.scale,
      });
      reset();
      setOpen(true);
    }).then((unlisten) => {
      stopContext = unlisten;
    });
    return () => stopContext?.();
  }, [reset]);

  useEffect(() => {
    let stopReset: (() => void) | undefined;
    void subscribe<null>("dwell_reset", () => {
      setOpen(false);
      reset();
    }).then((unlisten) => {
      stopReset = unlisten;
    });
    return () => stopReset?.();
  }, [reset]);

  // O overlay avisa quando o cursor sai/entra no gatilho. O timer de
  // cancelamento fica **aqui** e é pausado quando o cursor está na janela.
  useEffect(() => {
    let stopTrigger: (() => void) | undefined;
    void subscribe<DwellTriggerPayload>("dwell_trigger", (payload) => {
      overTrigger.current = payload.entered;
      if (payload.entered) {
        clearGrace();
      } else if (!overRoot.current) {
        ensureGrace();
      }
    }).then((unlisten) => {
      stopTrigger = unlisten;
    });
    return () => stopTrigger?.();
  }, [clearGrace, ensureGrace]);

  // Fonte confiável do hover: a posição global do cursor. O webview pode manter
  // estado de hover obsoleto ao ser escondido/mostrado de novo, então os eventos
  // de ponteiro sozinhos não bastam (bug "só funciona na primeira vez").
  useEffect(() => {
    if (!open || !geometry || !isTauriRuntime()) {
      return;
    }
    let cancelled = false;
    let timeout: number | null = null;

    const poll = async () => {
      const sample = await sampleDwellCursor(
        document.querySelector(".dwell-confirm"),
        geometry,
      );
      if (cancelled) {
        return;
      }
      if (sample) {
        if (sample.inside) {
          overRoot.current = true;
          clearGrace();
        } else {
          overRoot.current = false;
          if (!overTrigger.current) {
            ensureGrace();
          }
        }
        if (sample.overConfirm !== overConfirm.current) {
          overConfirm.current = sample.overConfirm;
          if (sample.overConfirm) {
            timerRef.current.start();
          } else {
            timerRef.current.stop();
          }
        }
      }
      timeout = window.setTimeout(poll, 90);
    };

    void poll();
    return () => {
      cancelled = true;
      if (timeout !== null) {
        window.clearTimeout(timeout);
      }
    };
  }, [open, geometry, clearGrace, ensureGrace]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        finish(false);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [finish]);

  return (
    <div
      className="dwell-window"
      data-token={token}
      onPointerEnter={() => {
        overRoot.current = true;
        clearGrace();
      }}
      onPointerMove={() => {
        // Rede de segurança: se o `pointerenter` se perder na troca de janela,
        // qualquer movimento dentro da confirmação pausa o cancelamento.
        overRoot.current = true;
        clearGrace();
      }}
      onPointerLeave={() => {
        overRoot.current = false;
        timer.stop();
        if (!overTrigger.current) {
          ensureGrace();
        }
      }}
      onPointerCancel={() => {
        overRoot.current = false;
        timer.stop();
        if (!overTrigger.current) {
          ensureGrace();
        }
      }}
    >
      <DwellPopover
        dwellMs={dwellMs}
        arming={timer.active}
        busy={busy}
        onArmStart={() => {
          overConfirm.current = true;
          timer.start();
        }}
        onArmStop={() => {
          overConfirm.current = false;
          timer.stop();
        }}
        onConfirm={() => {
          timer.stop();
          finish(true);
        }}
      />
    </div>
  );
}
