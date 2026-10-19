/** Adaptador IPC de sugestões e agentes. */
import { invokeSafe } from "./client";
import type {
  AssistantMode,
  ManualSuggestionContext,
  Suggestion,
} from "../../core/domain/assistant";

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
  /** Pede o rascunho curto (fast-first) antes do texto completo. */
  fastFirst: boolean;
  /**
   * Gera mesmo sem fala nova (botão “Sugerir agora”).
   *
   * Sem `force`, o core não gera de novo quando não houve fala nova — devolve
   * a última sugestão (BR-039).
   */
  force: boolean;
  /** Mensagem selecionada na Transcrição (sugestão manual — BR-041). */
  manualContext: ManualSuggestionContext | null;
}

/** Gera uma sugestão efêmera (não grava nada). */
export const suggestionGenerate = (
  params: SuggestionGenerateParams,
): Promise<Suggestion | undefined> =>
  invokeSafe<Suggestion>("suggestion_generate", params);

/** Grava a mensagem selecionada para a sugestão manual (não chama o LLM). */
export const prepareManualSuggestion = (params: {
  sessionId: string;
  speaker: "ME" | "THEY";
  text: string;
  atMs: number;
}): Promise<ManualSuggestionContext | undefined> =>
  invokeSafe<ManualSuggestionContext>("suggestion_prepare_manual", params);

/** Contexto manual atual da sessão (ou `null`). */
export const manualSuggestionContext = (
  sessionId: string,
): Promise<ManualSuggestionContext | null | undefined> =>
  invokeSafe<ManualSuggestionContext | null>("suggestion_manual_context", {
    sessionId,
  });

/** Limpa o contexto manual da sessão. */
export const clearManualSuggestionContext = (
  sessionId: string,
): Promise<void | undefined> =>
  invokeSafe<void>("suggestion_clear_manual_context", { sessionId });

/** Última sugestão em cache (sem chamar o LLM). */
export const suggestionLast = (
  sessionId: string,
): Promise<Suggestion | null | undefined> =>
  invokeSafe<Suggestion | null>("suggestion_last", { sessionId });

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
