/** Adaptador IPC do overlay. */
import { invokeSafe } from "./client";

export type Surface = "actionbar" | "hidden";

/** Ajusta a superfície da janela (barra/expandida/oculta). */
export const setSurface = (surface: Surface): Promise<void | undefined> =>
  invokeSafe<void>("overlay_set_surface", { surface });

/** Ajusta a largura lógica da barra medida pela UI (notch dinâmico). */
export const setActionbarWidth = (
  width: number,
): Promise<void | undefined> =>
  invokeSafe<void>("overlay_set_actionbar_width", { width });

/** Tipo de menu flutuante (janela `menu`). */
export type MenuKind = "agent" | "channel";

/** Retângulo em CSS px relativo à janela do overlay. */
export interface MenuRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Gatilho (eixo X) e corpo do notch (eixo Y) do menu. */
export interface MenuAnchor {
  trigger: MenuRect;
  notch: MenuRect;
}

/** Abre (ou reposiciona) o menu flutuante ancorado no gatilho. */
export const openMenu = (
  kind: MenuKind,
  anchor: MenuAnchor,
): Promise<void | undefined> =>
  invokeSafe<void>("menu_open", { kind, anchor });

/** Ajusta a altura do menu ao conteúdo medido (CSS px). */
export const resizeMenu = (height: number): Promise<void | undefined> =>
  invokeSafe<void>("menu_resize", { height });

/** Fecha o menu flutuante. */
export const closeMenu = (): Promise<void | undefined> =>
  invokeSafe<void>("menu_close");

/** Menu atualmente aberto (estado inicial do webview da janela `menu`). */
export const menuState = (): Promise<MenuKind | null | undefined> =>
  invokeSafe<MenuKind | null>("menu_state");

/** Alterna a visibilidade do overlay; retorna o novo estado. */
export const toggleOverlay = (): Promise<boolean | undefined> =>
  invokeSafe<boolean>("overlay_toggle");

/** Liga/desliga o modo stealth. */
export const setStealth = (enabled: boolean): Promise<void | undefined> =>
  invokeSafe<void>("overlay_set_stealth", { enabled });

/** Inicia o arraste da janela pelo usuário. */
export const startDrag = (): Promise<void | undefined> =>
  invokeSafe<void>("overlay_start_drag");

/** Abre (ou traz à frente) a janela de configurações. */
export const openSettingsWindow = (): Promise<void | undefined> =>
  invokeSafe<void>("settings_open");
