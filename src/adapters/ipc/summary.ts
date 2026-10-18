/** Adaptador IPC do resumo da sessão. */
import { invokeSafe } from "./client";
import type { MeetingSummary, SummaryResult } from "../../core/domain/assistant";

/** Resumo gravado da sessão (`null` se ainda não existe). */
export const summaryGet = (
  sessionId?: string | null,
): Promise<MeetingSummary | null | undefined> =>
  invokeSafe<MeetingSummary | null>("summary_get", {
    sessionId: sessionId ?? null,
  });

/**
 * Gera/atualiza o resumo.
 *
 * `mode` omitido = `"auto"` (incremental, sem chamar o LLM quando não há
 * novidade). `"manual"` regenera do zero.
 */
export const summaryGenerate = (
  sessionId: string | null,
  mode: "auto" | "manual" = "auto",
): Promise<SummaryResult | undefined> =>
  invokeSafe<SummaryResult>("summary_generate", {
    sessionId: sessionId ?? null,
    mode,
  });
