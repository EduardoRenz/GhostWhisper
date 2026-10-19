/** Adaptador IPC das janelas de painel (sugestões, chat/painel e histórico). */
import { invokeSafe } from "./client";
import type { PanelTab } from "../../core/domain/settings";

/** Labels de painel aceitos pelo core (janelas; o resumo é aba de `chat`). */
export type PanelLabel = "suggestions" | "chat" | "history";

/** Abre (ou foca) a janela do painel; `tab` abre o painel já na aba. */
export const panelOpen = (
  label: PanelLabel,
  tab?: PanelTab,
): Promise<void | undefined> =>
  invokeSafe<void>("panel_open", { label, tab: tab ?? null });

/** Abre/foca o painel da sessão na aba escolhida. */
export const panelOpenTab = (
  label: PanelLabel,
  tab: PanelTab,
): Promise<void | undefined> => panelOpen(label, tab);

/** Esconde a janela do painel. */
export const panelHide = (label: PanelLabel): Promise<void | undefined> =>
  invokeSafe<void>("panel_hide", { label });

/** Alterna mostrar/esconder o painel (botões do overlay). */
export async function togglePanel(label: PanelLabel): Promise<void> {
  const open = await panelIsOpen(label);
  await (open ? panelHide(label) : panelOpen(label));
}

/** Inicia o arraste da janela atual. */
export const panelStartDrag = (): Promise<void | undefined> =>
  invokeSafe<void>("panel_start_drag");

/** Informa se o painel está visível. */
export const panelIsOpen = (
  label: PanelLabel,
): Promise<boolean | undefined> =>
  invokeSafe<boolean>("panel_is_open", { label });
