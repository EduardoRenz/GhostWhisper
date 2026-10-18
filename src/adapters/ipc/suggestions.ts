/** Adaptador IPC de sugestões e agentes. */
import { invokeSafe } from "./client";
import type { AssistantMode, Suggestion } from "../../core/domain/assistant";

/** Lista os agentes embutidos + personalizados (com os prompts). */
export const assistantModesList = (): Promise<AssistantMode[] | undefined> =>
  invokeSafe<AssistantMode[]>("assistant_modes_list");

/** Parâmetros da geração efêmera da janela de sugestões. */
export interface SuggestionGenerateParams {
  [key: string]: unknown;
  sessionId: string | null;
  modeId: string | null;
  sinceMs: number | null;
  lastSuggestion: string | null;
  extraContext: string | null;
}

/** Gera uma sugestão efêmera (não grava nada). */
export const suggestionGenerate = (
  params: SuggestionGenerateParams,
): Promise<Suggestion | undefined> =>
  invokeSafe<Suggestion>("suggestion_generate", params);

/** Fixa uma sugestão já gerada na conversa. */
export const suggestionPin = (params: {
  sessionId: string;
  modeId: string;
  text: string;
  atMs: number;
}): Promise<Suggestion | undefined> =>
  invokeSafe<Suggestion>("suggestion_pin", params);

/** Gera e persiste uma sugestão no chat/atalho. */
export const llmInsight = (
  sessionId?: string | null,
): Promise<Suggestion | undefined> =>
  invokeSafe<Suggestion>("llm_insight", { sessionId: sessionId ?? null });
