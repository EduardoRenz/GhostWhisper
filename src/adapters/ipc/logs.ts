/** Adaptador IPC dos logs locais. */
import { invokeSafe } from "./client";

/** Caminho absoluto do arquivo de log. */
export const logsPath = (): Promise<string | undefined> =>
  invokeSafe<string>("logs_path");

/** Últimas linhas do log (o backend limita em 2.000). */
export const logsRecent = (limit = 300): Promise<string | undefined> =>
  invokeSafe<string>("logs_recent", { limit });
