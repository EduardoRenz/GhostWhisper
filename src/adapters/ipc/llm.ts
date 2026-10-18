/** Adaptador IPC de LLM (tradução e sugestões). */
import { invokeSafe } from "./client";

export { llmInsight } from "./suggestions";

/** Traduz um trecho para o idioma de destino configurado. */
export const llmTranslate = (text: string): Promise<string | undefined> =>
  invokeSafe<string>("llm_translate", { text });
