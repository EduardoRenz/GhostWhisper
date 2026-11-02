/** Adaptador IPC do contexto de tela (BR-105/BR-106). */
import { invokeSafe } from "./client";
import type {
  ScreenContextEvent,
  ScreenContextStatus,
} from "../../core/domain/screenContext";

/**
 * Snapshot do contexto para a pill ao abrir a AskBar.
 *
 * `null` quando a flag está desligada, a janela não tem contexto ou o
 * mecanismo falhou — a UI deriva o estado (`screenContextState`).
 */
export const screenContextSnapshot = (): Promise<
  ScreenContextEvent | null | undefined
> => invokeSafe<ScreenContextEvent | null>("screen_context_snapshot");

/** Status do recurso para a linha do card nas Configurações. */
export const screenContextStatus = (): Promise<ScreenContextStatus | undefined> =>
  invokeSafe<ScreenContextStatus>("screen_context_status");
