/** Adaptador IPC das janelas de painel (resumo e sugestões). */
import { invokeSafe } from "./client";

/** Labels de painel aceitos pelo core. */
export type PanelLabel = "summary" | "suggestions";

/** Abre (ou foca) a janela do painel. */
export const panelOpen = (label: PanelLabel): Promise<void | undefined> =>
  invokeSafe<void>("panel_open", { label });

/** Esconde a janela do painel. */
export const panelHide = (label: PanelLabel): Promise<void | undefined> =>
  invokeSafe<void>("panel_hide", { label });

/** Inicia o arraste da janela atual. */
export const panelStartDrag = (): Promise<void | undefined> =>
  invokeSafe<void>("panel_start_drag");

/** Informa se o painel está visível. */
export const panelIsOpen = (
  label: PanelLabel,
): Promise<boolean | undefined> =>
  invokeSafe<boolean>("panel_is_open", { label });
