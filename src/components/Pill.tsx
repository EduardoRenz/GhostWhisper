import type { ReactNode } from "react";

export type PillTone = "neutral" | "accent";

interface PillProps {
  children: ReactNode;
  /** Neutra (padrão) ou de destaque (cyan). */
  tone?: PillTone;
  /** Fonte mono/tabular (status e contagens). */
  mono?: boolean;
  /** Classe extra do caso de uso (ex.: `auto-control__count`). */
  className?: string;
  title?: string;
}

/**
 * Pill **padrão** do produto (LIVE, contagem, chip, ATIVO, contador).
 *
 * Centraliza borda, fundo, raio, tipografia e espaçamento; os casos de uso só
 * acrescentam o que é específico (largura mínima, alinhamento…). Substitui as
 * várias pills duplicadas no CSS.
 */
export function Pill({
  children,
  tone = "neutral",
  mono = false,
  className,
  title,
}: PillProps) {
  const classes = [
    "pill",
    `pill--${tone}`,
    mono ? "pill--mono" : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <span className={classes} title={title}>
      {children}
    </span>
  );
}
