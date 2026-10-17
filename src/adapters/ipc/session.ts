/** Adaptador IPC de sessão de transcrição. */
import { invokeSafe } from "./client";
import type { SessionChannel } from "../../core/domain/transcript";

/** Inicia uma sessão capturando os canais dados; retorna o id da sessão. */
export const sessionStart = (
  channels: SessionChannel[],
): Promise<string | undefined> =>
  invokeSafe<string>("session_start", { channels });

/** Encerra a sessão atual. */
export const sessionStop = (): Promise<void | undefined> =>
  invokeSafe<void>("session_stop");

/** Id da sessão ativa, se houver. */
export const sessionCurrent = (): Promise<string | null | undefined> =>
  invokeSafe<string | null>("session_current");
