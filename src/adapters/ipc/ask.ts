/** Adaptador IPC da barra de pesquisa (janela `ask`, BR-052). */
import { invokeSafe } from "./client";

/** Abre (ou foca) a barra de pesquisa. */
export const askOpen = (): Promise<void | undefined> =>
  invokeSafe<void>("ask_open");

/** Fecha a barra de pesquisa. */
export const askHide = (): Promise<void | undefined> =>
  invokeSafe<void>("ask_hide");

/** A barra está visível? */
export const askIsOpen = (): Promise<boolean | undefined> =>
  invokeSafe<boolean>("ask_is_open");

/** Inicia o arraste explícito da barra (única fonte de posição — BR-072). */
export const askStartDrag = (): Promise<void | undefined> =>
  invokeSafe<void>("ask_start_drag");
