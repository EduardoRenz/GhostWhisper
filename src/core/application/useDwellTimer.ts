import { useCallback, useEffect, useRef, useState } from "react";

/** Timer de permanência do drill-hover (um estágio). */
export interface DwellTimer {
  /** `true` enquanto o cursor está sobre o alvo e o tempo corre. */
  active: boolean;
  /** Inicia o timer; repetir enquanto ativo não reinicia a contagem. */
  start: () => void;
  /** Cancela e zera (saída/clique). */
  stop: () => void;
}

/**
 * Timer de permanência.
 *
 * Não anima nada: o preenchimento é CSS (`data-arming` + `--dwell-ms`). O hook só
 * dispara `onElapsed` quando o tempo completa — sem re-render por quadro.
 */
export function useDwellTimer(
  durationMs: number,
  onElapsed: () => void,
  enabled = true,
): DwellTimer {
  const [active, setActive] = useState(false);
  const timer = useRef<number | null>(null);
  const elapsed = useRef(onElapsed);
  elapsed.current = onElapsed;

  const stop = useCallback(() => {
    if (timer.current !== null) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
    setActive(false);
  }, []);

  const start = useCallback(() => {
    if (!enabled || timer.current !== null) {
      return;
    }
    setActive(true);
    timer.current = window.setTimeout(() => {
      timer.current = null;
      setActive(false);
      elapsed.current();
    }, durationMs);
  }, [durationMs, enabled]);

  useEffect(() => {
    if (!enabled) {
      stop();
    }
  }, [enabled, stop]);

  useEffect(
    () => () => {
      if (timer.current !== null) {
        window.clearTimeout(timer.current);
      }
    },
    [],
  );

  return { active, start, stop };
}
