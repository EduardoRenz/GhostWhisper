/**
 * Adaptador IPC — único ponto do frontend que conhece o Tauri.
 *
 * Fora do Tauri (ex.: `vite dev` no navegador) as chamadas viram no-op seguras,
 * permitindo desenvolver a UI sem o runtime nativo.
 */
import { invoke } from "@tauri-apps/api/core";

/** `true` quando o código roda dentro do runtime Tauri. */
export const isTauriRuntime = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Invoca um comando Tauri; retorna `undefined` fora do Tauri. */
export async function invokeSafe<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | undefined> {
  if (!isTauriRuntime()) {
    return undefined;
  }
  return invoke<T>(command, args);
}

/**
 * Assina o **foco** da janela atual (usado para recarregar dados frescos
 * quando o usuário abre uma janela que estava oculta).
 *
 * Devolve uma função de cancelamento; fora do Tauri é um no-op.
 */
export async function onWindowFocus(
  handler: (focused: boolean) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const unlisten = await getCurrentWindow().onFocusChanged((message) => {
    handler(message.payload);
  });
  return () => {
    void unlisten();
  };
}
