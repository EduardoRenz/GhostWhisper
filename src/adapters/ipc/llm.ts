/** Adaptador IPC de LLM (tradução e insights). */
import { invokeSafe } from "./client";
import type { InsightSuggestion } from "../../core/domain/transcript";

/** Traduz um trecho para o idioma de destino configurado. */
export const llmTranslate = (text: string): Promise<string | undefined> =>
  invokeSafe<string>("llm_translate", { text });

/**
 * Gera (e persiste) uma sugestão contextual.
 *
 * Sessão omitida = mais recente. A sugestão volta com `at_ms` (tempo desde o
 * início da sessão), para entrar na linha temporal.
 */
export const llmInsight = (
  sessionId?: string | null,
): Promise<InsightSuggestion | undefined> =>
  invokeSafe<InsightSuggestion>("llm_insight", { sessionId: sessionId ?? null });
