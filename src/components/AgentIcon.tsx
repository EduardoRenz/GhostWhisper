/**
 * Catálogo de ícones do agente (identidade visual — BR-037).
 *
 * Formas geométricas simples, desenhadas no projeto e herdando `currentColor`
 * (BR-008: nada de fonte de ícones de terceiros). A faísca reaproveita o
 * desenho já usado no botão do notch (BR-016: um conceito, um traço).
 */

import type { ReactNode } from "react";
import { AGENT_ICON_IDS, agentIconId } from "../core/domain/assistant";
import { SparkIcon } from "./TransportIcons";

export interface AgentIconProps {
  /** Id do ícone no catálogo (desconhecido → faísca). */
  icon: string;
  className?: string;
  /** Lado do quadrado (px). Padrão 18. */
  size?: number;
}

interface SvgProps {
  className?: string;
  size: number;
  children: ReactNode;
}

/** Casca comum: viewBox 24, traço consistente, invisível para leitores de tela. */
function Icon({ className, size, children }: SvgProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      width={size}
      height={size}
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  );
}

/** Traço padrão do catálogo. */
function Stroke({ children }: { children: ReactNode }) {
  return (
    <g
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {children}
    </g>
  );
}

type Glyph = (props: { className?: string; size: number }) => ReactNode;

/** Catálogo: id → desenho. */
const GLYPHS: Record<string, Glyph> = {
  // Faísca: o mesmo desenho do botão do notch (BR-016).
  sparkle: ({ className, size }) => (
    <SparkIcon className={className} size={size} />
  ),
  // Duas pessoas: reunião/entrevista.
  users: ({ className, size }) => (
    <Icon className={className} size={size}>
      <Stroke>
        <circle cx="9.2" cy="8.4" r="3.1" />
        <path d="M3.6 19.2c.5-3 2.8-4.8 5.6-4.8s5.1 1.8 5.6 4.8" />
        <path d="M15.4 6.1a3 3 0 0 1 0 5.6" />
        <path d="M17 14.9c2 .7 3.3 2.3 3.5 4.3" />
      </Stroke>
    </Icon>
  ),
  // Chavetas: entrevista técnica.
  braces: ({ className, size }) => (
    <Icon className={className} size={size}>
      <Stroke>
        <path d="M9.6 3.8c-1.9 0-2.6.9-2.6 2.4v1.5c0 1.3-.6 2-1.8 2.4v.2c1.2.4 1.8 1.1 1.8 2.4v1.5c0 1.5.7 2.4 2.6 2.4" />
        <path d="M14.4 3.8c1.9 0 2.6.9 2.6 2.4v1.5c0 1.3.6 2 1.8 2.4v.2c-1.2.4-1.8 1.1-1.8 2.4v1.5c0 1.5-.7 2.4-2.6 2.4" />
      </Stroke>
    </Icon>
  ),
  // Chevrons de código: LeetCode.
  code: ({ className, size }) => (
    <Icon className={className} size={size}>
      <Stroke>
        <path d="M8.6 8.2 4.4 12l4.2 3.8" />
        <path d="M15.4 8.2 19.6 12l-4.2 3.8" />
        <path d="M13.4 5.2 10.6 18.8" />
      </Stroke>
    </Icon>
  ),
  // Camadas: system design.
  layers: ({ className, size }) => (
    <Icon className={className} size={size}>
      <Stroke>
        <path d="M12 3.4 3.6 7.6 12 11.8l8.4-4.2z" />
        <path d="M3.6 12.2 12 16.4l8.4-4.2" />
        <path d="M3.6 16.6 12 20.8l8.4-4.2" />
      </Stroke>
    </Icon>
  ),
  // Livro: estudo/pesquisa.
  book: ({ className, size }) => (
    <Icon className={className} size={size}>
      <Stroke>
        <path d="M4.4 5.2c2.6-.9 5-.7 7.6.8v13c-2.6-1.5-5-1.7-7.6-.8z" />
        <path d="M19.6 5.2c-2.6-.9-5-.7-7.6.8v13c2.6-1.5 5-1.7 7.6-.8z" />
      </Stroke>
    </Icon>
  ),
  // Pasta de trabalho: vendas/negócios.
  briefcase: ({ className, size }) => (
    <Icon className={className} size={size}>
      <Stroke>
        <rect x="3.4" y="7.4" width="17.2" height="12" rx="2" />
        <path d="M9 7.4V5.9c0-.9.7-1.6 1.6-1.6h2.8c.9 0 1.6.7 1.6 1.6v1.5" />
        <path d="M3.4 12.4h17.2" />
      </Stroke>
    </Icon>
  ),
  // Terminal: shell/devops.
  terminal: ({ className, size }) => (
    <Icon className={className} size={size}>
      <Stroke>
        <rect x="3.2" y="4.6" width="17.6" height="14.8" rx="2" />
        <path d="M7.4 9.6 10 12l-2.6 2.4" />
        <path d="M12.4 15h4.2" />
      </Stroke>
    </Icon>
  ),
};

/** Rótulo legível de cada ícone do catálogo (tooltip e seletor). */
export const AGENT_ICON_LABELS: Record<string, string> = {
  sparkle: "Faísca",
  users: "Pessoas",
  braces: "Chavetas",
  code: "Código",
  layers: "Camadas",
  book: "Livro",
  briefcase: "Pasta",
  terminal: "Terminal",
};

/** Ícone do agente — herda a cor do chamador (`currentColor`). */
export function AgentIcon({ icon, className, size = 18 }: AgentIconProps) {
  const id = agentIconId({ icon });
  const glyph = GLYPHS[id] ?? GLYPHS.sparkle;
  return <>{glyph?.({ className, size })}</>;
}

/** Ids do catálogo (para menus de escolha). */
export const AGENT_ICON_OPTIONS = AGENT_ICON_IDS;
