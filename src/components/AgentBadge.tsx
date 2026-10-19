/**
 * Pill de identificação do agente (ícone + nome na cor do agente — BR-037).
 *
 * A cor é um **token** do design system (`--oh-agent-*`), nunca hex solto: o
 * tema continua sendo a fonte da verdade e o componente não conhece valores.
 */

import type { CSSProperties } from "react";
import { agentColorToken, agentIconId } from "../core/domain/assistant";
import { AgentIcon } from "./AgentIcon";

export interface AgentBadgeProps {
  /** Agente (usa nome, ícone e cor); `null` mostra um rótulo neutro. */
  mode: { name: string; icon?: string; color?: string } | null | undefined;
  /** Lado do ícone (px). Padrão 13. */
  size?: number;
  className?: string;
  /** Texto alternativo quando não há agente. */
  fallback?: string;
}

/** Pill com ícone e nome do agente, na cor dele. */
export function AgentBadge({
  mode,
  size = 13,
  className,
  fallback = "Agente",
}: AgentBadgeProps) {
  const token = agentColorToken(mode);
  const style = {
    "--agent-color": `var(--oh-agent-${token})`,
  } as CSSProperties;
  return (
    <span
      className={["agent-badge", className ?? ""].filter(Boolean).join(" ")}
      style={style}
      title={mode?.name}
    >
      <AgentIcon icon={agentIconId(mode)} size={size} />
      <span className="agent-badge__name">{mode?.name ?? fallback}</span>
    </span>
  );
}
