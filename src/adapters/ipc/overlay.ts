/** Adaptador IPC do overlay. */
import { invokeSafe } from "./client";

export type Surface = "actionbar" | "hidden";

/** Ajusta a superfície da janela (barra/expandida/oculta). */
export const setSurface = (surface: Surface): Promise<void | undefined> =>
  invokeSafe<void>("overlay_set_surface", { surface });

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
