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
  /** Instante absoluto da pergunta (epoch, ms) — a hora de parede. */
  created_at_ms: number;
  status: JournalStatus;
  request_source: JournalRequestSource;
  /** Provedor que respondeu. */
  provider: string | null;
  /** Modelo que respondeu. */
  model: string | null;
  /** Mensagem de erro (quando `status = error`). */
  error: string | null;
  /**
   * Resumo do par pergunta/resposta (até 240 caracteres) — o que a IA já
   * respondeu. Alimenta o contexto das perguntas seguintes; a UI não usa.
   */
  summary?: string | null;
}

/** Progresso de uma entrada (evento `journal`). */
export interface JournalUpdate {
  entry_id: string;
  phase: "pending" | "streaming" | "answered" | "error";
  /** Texto acumulado da resposta. */
  text: string;
  error: string | null;
}

/**
 * Origem da entrada, como o cabeçalho da aba mostra (BR-052): **texto**,
 * **print** ou os dois.
 */
export type JournalOrigin = "text" | "screenshot" | "both" | "preset";

/** Rótulo da origem (a "legenda" do topo da entrada). */
export const JOURNAL_ORIGIN_LABELS: Record<JournalOrigin, string> = {
  text: "Pesquisa",
  screenshot: "Screenshot",
  both: "Screenshot + Texto",
  preset: "Pergunta pronta",
};

/** Rótulo curto da origem (compatibilidade com a lista plana). */
export const JOURNAL_SOURCE_LABELS: Record<JournalRequestSource, string> = {
  search: "Pesquisa",
  preset: "Pergunta pronta",
  screenshot: "Screenshot",
};

/**
 * A origem de uma entrada: `screenshot` carrega o print; com pergunta junto
 * vira **os dois**. A pergunta pode ser vazia (print puro — a IA decide).
 */
export function journalOrigin(entry: JournalEntry): JournalOrigin {
  if (entry.request_source === "screenshot") {
    return entry.question.trim() === "" ? "screenshot" : "both";
  }
  return entry.request_source === "preset" ? "preset" : "text";
}

/** A entrada tem pergunta do usuário? (print puro não tem — não mostra "Q:"). */
export function hasJournalQuestion(entry: JournalEntry): boolean {
  return entry.question.trim() !== "";
}
