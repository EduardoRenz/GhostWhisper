/** Tipos de atalhos globais (espelham `domain::shortcuts`). */

export type ShortcutAction =
  | "toggle_overlay"
  | "toggle_session"
  | "toggle_stealth"
  | "request_insight";

export interface ShortcutBinding {
  action: ShortcutAction;
  accelerator: string;
}

export interface ShortcutConflict {
  action: ShortcutAction;
  accelerator: string;
  reason: string;
}

export interface ShortcutInfo {
  action: ShortcutAction;
  label: string;
  default_accelerator: string;
}

/** Ações na ordem de exibição. */
export const SHORTCUT_ACTIONS: ShortcutAction[] = [
  "toggle_overlay",
  "toggle_session",
  "toggle_stealth",
  "request_insight",
];

/** Vínculos padrão (espelham `default_bindings`). */
export const defaultBindings = (): ShortcutBinding[] => [
  { action: "toggle_overlay", accelerator: "CommandOrControl+Shift+O" },
  { action: "toggle_session", accelerator: "CommandOrControl+Shift+S" },
  { action: "toggle_stealth", accelerator: "CommandOrControl+Shift+H" },
  { action: "request_insight", accelerator: "CommandOrControl+Shift+I" },
];
