/**
 * Pill com o **modelo** que respondeu (BR-051).
 *
 * Mostra o modelo (o que o usuário precisa saber: `deepseek-reasoner`) e leva
 * no *tooltip* a fonte completa ("DeepSeek · deepseek-reasoner"), com a marca
 * de fallback quando a resposta veio de uma fonte secundária.
 */

import { BoltIcon } from "./icons";
import type { LlmSourceUsage } from "../core/application/useLlmSource";

export interface LlmSourceBadgeProps {
  /** Último uso da fonte (pode ser `null` enquanto não houve chamada). */
  usage: LlmSourceUsage | null;
  /** Classe extra do caso de uso. */
  className?: string;
}

/** Pill de modelo da fonte que respondeu. */
export function LlmSourceBadge({ usage, className }: LlmSourceBadgeProps) {
  if (!usage) return null;
  const title = [
    `${usage.label} · ${usage.model}`,
    usage.is_fallback ? "fallback" : null,
  ]
    .filter(Boolean)
    .join(" · ");
  const classes = [
    "llm-source-badge",
    usage.is_fallback ? "llm-source-badge--fallback" : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <span className={classes} title={title}>
      <BoltIcon size={11} />
      <span className="llm-source-badge__model">{usage.model}</span>
    </span>
  );
}
