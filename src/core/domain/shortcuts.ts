/** Tipos de atalhos globais (espelham `domain::shortcuts`). */

export type ShortcutAction =
  | "focus_ask"
  | "capture_screen"
  | "toggle_overlay"
  | "toggle_session"
  | "recording_toggle"
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
  "focus_ask",
  "capture_screen",
  "toggle_overlay",
  "toggle_session",
  "recording_toggle",
  "toggle_stealth",
  "request_insight",
];

/** Vínculos padrão (espelham `default_bindings`). */
export const defaultBindings = (): ShortcutBinding[] => [
  { action: "focus_ask", accelerator: "CommandOrControl+K" },
  { action: "capture_screen", accelerator: "Alt+C" },
  { action: "toggle_overlay", accelerator: "CommandOrControl+Shift+O" },
  { action: "toggle_session", accelerator: "CommandOrControl+Shift+S" },
  { action: "recording_toggle", accelerator: "CommandOrControl+Shift+R" },
  { action: "toggle_stealth", accelerator: "CommandOrControl+Shift+H" },
  { action: "request_insight", accelerator: "CommandOrControl+Shift+I" },
];

/** Rótulos de exibição dos modificadores (chaves em minúsculas). */
const MODIFIER_LABELS: Record<string, string> = {
  commandorcontrol: "Ctrl",
  cmdorctrl: "Ctrl",
  command: "Ctrl",
  cmd: "Ctrl",
  control: "Ctrl",
  ctrl: "Ctrl",
  shift: "Shift",
  alt: "Alt",
  option: "Alt",
  altgr: "AltGr",
  super: "Super",
  meta: "Super",
  win: "Super",
};

/**
 * Quebra um *accelerator* do Tauri em tokens (puro).
 *
 * `"CommandOrControl+Shift+S"` → `["CommandOrControl", "Shift", "S"]`. Tokens
 * vazios (ex.: `"Ctrl++A"`) são descartados.
 */
export function parseAccelerator(accelerator: string): string[] {
  return accelerator
    .split("+")
    .map((token) => token.trim())
    .filter((token) => token.length > 0);
}

/**
 * Converte um *accelerator* nos *badges* exibidos ao lado da ação.
 *
 * Modificadores viram rótulos curtos (`CommandOrControl`→`Ctrl`); uma tecla de
 * um caractere vira maiúscula (`s`→`S`); o resto é preservado (`F1`, `Enter`).
 * Entrada vazia devolve `[]` (a UI não desenha nada).
 */
export function acceleratorBadges(accelerator: string): string[] {
  return parseAccelerator(accelerator).map((token) => {
    const modifier = MODIFIER_LABELS[token.toLowerCase()];
    if (modifier) {
      return modifier;
    }
    return token.length === 1 ? token.toUpperCase() : token;
  });
}
