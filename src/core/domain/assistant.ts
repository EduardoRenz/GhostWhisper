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
  /** Ícone do agente (catálogo — BR-037). */
  icon: string;
  /** Cor do agente (token do design system — BR-037). */
  color: string;
  /**
   * Fonte de IA vinculada ao agente (BR-048).
   *
   * `undefined` = usa a fonte primária. A UI de vínculo é da PRD seguinte.
   */
  source_id?: string | null;
}

/** IDs reservados dos agentes embutidos. */
export const BUILTIN_MODE_IDS = [
  "meeting",
  "technical_interview",
  "leetcode",
  "system_design",
] as const;

/** Catálogo de ícones do agente (espelha `domain::assistant`). */
export const AGENT_ICON_IDS = [
  "sparkle",
  "users",
  "braces",
  "code",
  "layers",
  "book",
  "briefcase",
  "terminal",
] as const;

/** Paleta de cores do agente (tokens do design system — espelha o core). */
export const AGENT_COLOR_TOKENS = [
  "cyan",
  "violet",
  "emerald",
  "coral",
  "slate",
] as const;

/** Ícone padrão de um agente sem identidade definida. */
export const DEFAULT_AGENT_ICON = "sparkle";

/** Cor padrão de um agente sem identidade definida. */
export const DEFAULT_AGENT_COLOR = "slate";

/** `true` quando o id de ícone existe no catálogo. */
export const isAgentIconId = (icon: string): boolean =>
  (AGENT_ICON_IDS as readonly string[]).includes(icon);

/** `true` quando o token de cor existe na paleta. */
export const isAgentColorToken = (color: string): boolean =>
  (AGENT_COLOR_TOKENS as readonly string[]).includes(color);

/** Ícone do agente com fallback (settings antigas não têm o campo). */
export const agentIconId = (
  mode: { icon?: string | null } | null | undefined,
): string =>
  mode && mode.icon && isAgentIconId(mode.icon) ? mode.icon : DEFAULT_AGENT_ICON;

/** Cor do agente com fallback (settings antigas não têm o campo). */
export const agentColorToken = (
  mode: { color?: string | null } | null | undefined,
): string =>
  mode && mode.color && isAgentColorToken(mode.color)
    ? mode.color
    : DEFAULT_AGENT_COLOR;

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

/** Fase da sugestão (BR-038): resposta curta ou completa. */
export type SuggestionPhase = "quick" | "full";

/** Origem da geração (BR-041/BR-042). */
export type SuggestionTrigger = "auto" | "now" | "manual";

/** Mensagem selecionada na Transcrição para uma sugestão manual (BR-041). */
export interface ManualSuggestionContext {
  session_id: string;
  speaker: "ME" | "THEY";
  /** Texto completo do segmento. */
  text: string;
  /** Tempo desde o início da sessão (ms). */
  at_ms: number;
}

/** Snapshot leve da mensagem que originou uma sugestão manual. */
export interface SuggestionSource {
  speaker: "ME" | "THEY";
  at_ms: number;
  preview: string;
}

/** Sugestão gerada (`ports::suggestion_generator::Suggestion`). */
export interface Suggestion {
  session_id: string;
  mode_id: string;
  mode_name: string;
  /** Snapshot do ícone do agente (BR-037). */
  mode_icon: string;
  /** Snapshot da cor do agente (BR-037). */
  mode_color: string;
  /** UUID da geração; quick/full/deltas compartilham o mesmo id. */
  generation_id: string;
  /** Origem da geração (BR-041/BR-042). */
  trigger: SuggestionTrigger;
  /** Snapshot da mensagem de origem (só na manual). */
  source: SuggestionSource | null;
  /** Fase da sugestão (BR-038). */
  phase: SuggestionPhase;
  /** Título curto gerado pelo agente (`Subject:`); vazio = derive do texto. */
  title: string;
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
