/** Assinatura de eventos Tauri (no-op fora do runtime nativo). */
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { isTauriRuntime } from "./client";

/** Assina um evento do core; devolve a função de cancelamento. */
export async function subscribe<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (!isTauriRuntime()) {
    return () => {};
  }
  return listen<T>(event, (message) => handler(message.payload));
}
