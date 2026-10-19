/**
 * Regras puras do drill-hover (BR-036).
 *
 * O posicionamento da janela de confirmação vive no core (área útil do monitor);
 * aqui ficam só a faixa/duração e o rótulo.
 */
import {
  DEFAULT_DWELL_MS,
  MAX_DWELL_MS,
  MIN_DWELL_MS,
} from "./settings";

export { DEFAULT_DWELL_MS, MAX_DWELL_MS, MIN_DWELL_MS };

/** Traz a duração para a faixa aceita (ms inteiros). */
export function clampDwellMs(ms: number): number {
  if (!Number.isFinite(ms)) {
    return DEFAULT_DWELL_MS;
  }
  return Math.min(MAX_DWELL_MS, Math.max(MIN_DWELL_MS, Math.round(ms)));
}

/** Rótulo curto da duração (`0,5 s`). */
export function dwellLabel(ms: number): string {
  const seconds = clampDwellMs(ms) / 1000;
  return `${seconds.toFixed(1).replace(".", ",")} s`;
}
