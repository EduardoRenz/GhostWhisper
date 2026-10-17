/** Adaptador IPC de LLM (tradução e insights). */
import { invokeSafe } from "./client";

/** Traduz um trecho para o idioma de destino configurado. */
export const llmTranslate = (text: string): Promise<string | undefined> =>
  invokeSafe<string>("llm_translate", { text });

/** Gera uma sugestão contextual (sessão omitida = mais recente). */
export const llmInsight = (
  sessionId?: string | null,
): Promise<string | undefined> =>
  invokeSafe<string>("llm_insight", { sessionId: sessionId ?? null });
