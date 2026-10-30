/** Adaptador IPC de leitura/gestão do banco (histórico, linha temporal e IA). */
import { invokeSafe } from "./client";
import type {
  ConversationMessage,
  SessionSummary,
  TimelineEntry,
  TranscriptSegment,
} from "../../core/domain/transcript";

/**
 * Transcrições persistidas de uma sessão, em ordem temporal.
 *
 * Usado ao abrir/retomar uma sessão: a linha do tempo é recarregada do banco
 * em vez de começar vazia (BR-019).
 */
export const sessionTranscriptions = (
  sessionId: string,
): Promise<TranscriptSegment[] | undefined> =>
  invokeSafe<TranscriptSegment[]>("db_session_transcriptions", { sessionId });

/** Sessões mais recentes (histórico). */
export const listSessions = (
  limit?: number,
): Promise<SessionSummary[] | undefined> =>
  invokeSafe<SessionSummary[]>("db_list_sessions", { limit: limit ?? null });

/**
 * Busca sessões pelo **título** (caixa/acento ignorados, BR-103).
 *
 * Consulta vazia devolve vazio; a lista sem filtro vem de `listSessions`.
 */
export const searchSessions = (
  query: string,
  limit?: number,
): Promise<SessionSummary[] | undefined> =>
  invokeSafe<SessionSummary[]>("db_search_sessions", {
    query,
    limit: limit ?? null,
  });

/** Linha temporal da sessão: falas + sugestões já ordenadas. */
export const sessionTimeline = (
  sessionId: string,
): Promise<TimelineEntry[] | undefined> =>
  invokeSafe<TimelineEntry[]>("db_session_timeline", { sessionId });

/** Sugestões (mensagens da IA) já gravadas da sessão. */
export const sessionMessages = (
  sessionId: string,
): Promise<ConversationMessage[] | undefined> =>
  invokeSafe<ConversationMessage[]>("db_session_messages", { sessionId });

/** Renomeia a sessão (título do histórico). */
export const renameSession = (
  id: string,
  subject: string,
): Promise<void | undefined> =>
  invokeSafe<void>("session_rename", { id, subject });

/** Exclui a sessão e, junto, falas e sugestões. */
export const deleteSession = (id: string): Promise<void | undefined> =>
  invokeSafe<void>("session_delete", { id });
