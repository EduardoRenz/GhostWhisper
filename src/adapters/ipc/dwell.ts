/** Adaptador IPC da janela de confirmação do drill-hover (BR-036). */
import { invokeSafe } from "./client";

/** Retângulo do gatilho em CSS px, relativo à janela do overlay. */
export interface DwellAnchor {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Abre a confirmação ancorada no gatilho. */
export const dwellShow = (
  anchor: DwellAnchor,
  dwellMs: number,
): Promise<void | undefined> =>
  invokeSafe<void>("dwell_show", { anchor, dwellMs });

/** Fecha a confirmação sem resultado. */
export const dwellHide = (): Promise<void | undefined> =>
  invokeSafe<void>("dwell_hide");

/** Duração atual do segundo estágio (para a janela `dwell`). */
export const dwellContext = (): Promise<number | null | undefined> =>
  invokeSafe<number | null>("dwell_context");

/** Resultado da confirmação. */
export const dwellResult = (
  confirmed: boolean,
): Promise<void | undefined> =>
  invokeSafe<void>("dwell_result", { confirmed });

/** Informa a janela `dwell` se o cursor está no gatilho (grace local). */
export const dwellTriggerHover = (
  entered: boolean,
): Promise<void | undefined> =>
  invokeSafe<void>("dwell_trigger_hover", { entered });
