/** Adaptador IPC do diário de perguntas (aba "Sessão", BR-052). */
import { invokeSafe } from "./client";
import type { JournalEntry, JournalRequestSource } from "../../core/domain/journal";

/** Imagem anexada (base64, sem o prefixo `data:`). */
export interface AttachedImage {
  mime: string;
  data: string;
}

/** Pedido de pergunta. */
export interface JournalAskRequest {
  /** Sessão alvo (vazia = a ativa ou a mais recente). */
  session_id?: string | null;
  question: string;
  source?: JournalRequestSource;
  images?: AttachedImage[];
}

/** Resultado de uma pergunta. */
export interface JournalAskOutcome {
  entry_id: string;
  answer: string;
}

/** Diário de uma sessão, em ordem cronológica. */
export const journalList = (
  sessionId?: string | null,
): Promise<JournalEntry[] | undefined> =>
  invokeSafe<JournalEntry[]>("journal_list", { sessionId: sessionId ?? null });

/** Uma entrada do diário. */
export const journalGet = (id: string): Promise<JournalEntry | undefined> =>
  invokeSafe<JournalEntry>("journal_get", { id });

/** Busca full-text no diário da sessão. */
export const journalSearch = (
  sessionId: string | null,
  query: string,
  limit?: number,
): Promise<JournalEntry[] | undefined> =>
  invokeSafe<JournalEntry[]>("journal_search", {
    sessionId,
    query,
    limit: limit ?? null,
  });

/** Imagens de uma entrada (miniatura do print). */
export const journalImages = (
  entryId: string,
): Promise<
  { id: string; entry_id: string; mime: string; data: number[] }[] | undefined
> => invokeSafe("journal_images", { entryId });

/** Remove uma entrada do diário. */
export const journalDelete = (id: string): Promise<void | undefined> =>
  invokeSafe<void>("journal_delete", { id });

/** Faz a pergunta (a resposta chega pelo evento `journal`). */
export const journalAsk = (
  request: JournalAskRequest,
): Promise<JournalAskOutcome | undefined> =>
  invokeSafe<JournalAskOutcome>("journal_ask", { request });

/** Cancela uma pergunta em curso. */
export const journalCancel = (id: string): Promise<void | undefined> =>
  invokeSafe<void>("journal_cancel", { id });
