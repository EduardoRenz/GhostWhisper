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

/** Suspende a captura mantendo a sessão aberta (BR-019). */
export const sessionPause = (): Promise<void | undefined> =>
  invokeSafe<void>("session_pause");

/** Volta a capturar na sessão pausada (BR-019). */
export const sessionResume = (): Promise<void | undefined> =>
  invokeSafe<void>("session_resume");

/** Troca os canais capturados sem encerrar a sessão. */
export const sessionSetChannels = (
  channels: SessionChannel[],
): Promise<void | undefined> =>
  invokeSafe<void>("session_set_channels", { channels });

/** Continua uma sessão existente (reativa se encerrada) e captura nela. */
export const sessionContinue = (
  id: string,
  channels: SessionChannel[],
): Promise<string | undefined> =>
  invokeSafe<string>("session_continue", { id, channels });

/** Id da sessão ativa, se houver. */
export const sessionCurrent = (): Promise<string | null | undefined> =>
  invokeSafe<string | null>("session_current");

/** Troca o idioma de origem da transcrição em tempo real (BR-034). */
export const sessionSetLanguage = (
  language: string,
): Promise<void | undefined> =>
  invokeSafe<void>("session_set_language", { language });
