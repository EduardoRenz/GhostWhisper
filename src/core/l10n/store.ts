/**
 * Store global do idioma da interface.
 *
 * As janelas Tauri são processos de webview independentes: cada uma carrega as
 * settings e aplica o idioma aqui. `useSyncExternalStore` garante re-render
 * quando o idioma muda (ex.: save em Configurações → `settings_changed`).
 */

import { DEFAULT_UI_LANGUAGE, type UiLanguage } from "./language";

let currentLanguage: UiLanguage = DEFAULT_UI_LANGUAGE;
const listeners = new Set<() => void>();

/** Idioma atual do store. */
export function getUiLanguage(): UiLanguage {
  return currentLanguage;
}

/** Troca o idioma ativo e avisa os componentes. */
export function setUiLanguage(language: UiLanguage | null | undefined): void {
  if (!language || language === currentLanguage) {
    return;
  }
  currentLanguage = language;
  for (const listener of listeners) {
    listener();
  }
}

/** Assina as trocas de idioma (para `useSyncExternalStore`). */
export function subscribeUiLanguage(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
