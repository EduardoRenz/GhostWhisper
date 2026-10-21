/** Adaptador IPC do modal "Sessão Finalizada" (BR-068). */
import { invokeSafe } from "./client";
import type { SessionEnd } from "../../core/domain/recording";

/** Abre o modal para a sessão encerrada. */
export const sessionEndOpen = (
  sessionId: string,
): Promise<void | undefined> =>
  invokeSafe<void>("session_end_open", { sessionId });

/** Contexto do modal (recarregável). */
export const sessionEndContext = (): Promise<SessionEnd | null | undefined> =>
  invokeSafe<SessionEnd | null>("session_end_context");

/** Fecha o modal e limpa o contexto. */
export const sessionEndClose = (): Promise<void | undefined> =>
  invokeSafe<void>("session_end_close");
