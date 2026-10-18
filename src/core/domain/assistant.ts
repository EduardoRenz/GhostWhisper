/** Agentes/modos de sugestão e resumo (espelham `domain::assistant`/`overview`). */

/** Agente de sugestão (embutido ou personalizado). */
export interface AssistantMode {
  id: string;
  name: string;
  category: string;
  /** Contrato de respostas escritas / critérios de domínio. */
  system_prompt: string;
  /** Contrato da sugestão ao vivo. */
  suggestion_prompt: string;
  /** Contexto fixo do agente. */
  context: string;
}

/** IDs reservados dos agentes embutidos. */
export const BUILTIN_MODE_IDS = [
  "meeting",
  "technical_interview",
  "leetcode",
  "system_design",
] as const;

/** Limite de caracteres de cada prompt do agente (espelha o core). */
export const MODE_PROMPT_MAX_CHARS = 16_384;
/** Limite do nome de exibição do agente. */
export const MODE_NAME_MAX_CHARS = 120;

/** `true` quando o id pertence a um agente embutido. */
export const isBuiltinMode = (id: string): boolean =>
  (BUILTIN_MODE_IDS as readonly string[]).includes(id);

/** Resumo persistido da sessão (`domain::overview::MeetingSummary`). */
export interface MeetingSummary {
  session_id: string;
  subject: string | null;
  markdown: string;
  summarized_from_ms: number;
  summarized_until_ms: number;
  updated_at_ms: number;
}

/** Resultado da geração do resumo. */
export interface SummaryResult {
  summary: MeetingSummary;
  /** `true` quando o LLM foi chamado e o conteúdo mudou. */
  updated: boolean;
  /** `true` quando ainda há trechos a processar. */
  has_more: boolean;
}

/** Sugestão gerada (`ports::suggestion_generator::Suggestion`). */
export interface Suggestion {
  session_id: string;
  mode_id: string;
  mode_name: string;
  text: string;
  at_ms: number;
  context_until_ms: number;
  message_id: string | null;
  skipped: boolean;
}

/** Rótulo de um agente (fallback para sugestão genérica). */
export const modeLabel = (
  mode: Pick<AssistantMode, "name"> | null | undefined,
  fallback = "Sugestão da IA",
): string => mode?.name ?? fallback;

/** Título do tópico de um resumo: `subject` do modelo ou o primeiro heading. */
export function summaryTopic(
  summary: MeetingSummary | null | undefined,
): string {
  const subject = summary?.subject?.trim();
  if (subject) {
    return subject;
  }
  const heading = summary?.markdown.match(/^#{1,6}\s+(.+)$/m)?.[1]?.trim();
  return heading || "Resumo sem título";
}
