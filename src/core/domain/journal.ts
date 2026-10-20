/**
 * Tipos do diário de perguntas (aba "Sessão", BR-052).
 *
 * Espelham `domain/journal.rs` no core Rust.
 */

/** Estado de uma entrada do diário. */
export type JournalStatus = "pending" | "answered" | "error" | "cancelled";

/** De onde veio a pergunta. */
export type JournalRequestSource = "search" | "preset" | "screenshot";

/** Uma entrada do diário: a pergunta e a resposta. */
export interface JournalEntry {
  id: string;
  session_id: string;
  question: string;
  /** Resposta da IA (`null` enquanto `pending`). */
  answer: string | null;
  /** Milissegundos desde o início da sessão (BR-019). */
  asked_at_ms: number;
  answered_at_ms: number | null;
  status: JournalStatus;
  request_source: JournalRequestSource;
  /** Provedor que respondeu. */
  provider: string | null;
  /** Modelo que respondeu. */
  model: string | null;
  /** Mensagem de erro (quando `status = error`). */
  error: string | null;
}

/** Progresso de uma entrada (evento `journal`). */
export interface JournalUpdate {
  entry_id: string;
  phase: "pending" | "streaming" | "answered" | "error";
  /** Texto acumulado da resposta. */
  text: string;
  error: string | null;
}

/** Rótulo curto da origem (mostrado na entrada). */
export const JOURNAL_SOURCE_LABELS: Record<JournalRequestSource, string> = {
  search: "Pesquisa",
  preset: "Pergunta pronta",
  screenshot: "Print",
};
