/**
 * Ícones da interface (SVG desenhados no projeto).
 *
 * Substituem rótulos de texto na barra do overlay (BR-032). Não copiamos a fonte
 * Material Icons do Perssua nem qualquer asset de terceiros (BR-008): formas
 * geométricas simples, `currentColor` para herdar a cor e `stroke` para um traço
 * consistente. Transporte (play/pause/stop) vive em `TransportIcons.tsx`.
 */

import type { ReactNode } from "react";

export interface IconProps {
  className?: string;
  /** Lado do quadrado (px). Padrão 18. */
  size?: number;
}

interface SvgProps {
  className?: string;
  size: number;
  children: ReactNode;
}

/** Casca comum: viewBox 24, sem foco e invisível para leitores de tela. */
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

/** Engrenagem: abre as Configurações. */
export function GearIcon({ className, size = 18 }: IconProps) {
  const ticks = [0, 45, 90, 135, 180, 225, 270, 315];
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
      >
        <circle cx="12" cy="12" r="6.3" />
        <circle cx="12" cy="12" r="2.3" />
        {ticks.map((angle) => (
          <line
            key={angle}
            x1="12"
            y1="3.1"
            x2="12"
            y2="5.1"
            transform={`rotate(${angle} 12 12)`}
          />
        ))}
      </g>
    </Icon>
  );
}

/** Balão com linhas: a aba/atalho "Transcrição". */
export function TranscriptIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M5 4.5h14A1.5 1.5 0 0 1 20.5 6v8.5A1.5 1.5 0 0 1 19 16H9l-4 3.3V16H5A1.5 1.5 0 0 1 3.5 14.5V6A1.5 1.5 0 0 1 5 4.5Z" />
        <path d="M7.5 8.5h9" />
        <path d="M7.5 11.8h6" />
      </g>
    </Icon>
  );
}

/** Documento com linhas: a aba/atalho "Resumo". */
export function SummaryIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M6 3.5h7.5L18 7.9V20.5H6Z" />
        <path d="M13.4 3.6V8h4.4" />
        <path d="M8.8 12h6.4" />
        <path d="M8.8 15.2h6.4" />
      </g>
    </Icon>
  );
}

/** Relógio com seta: "Histórico". */
export function HistoryIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M4.2 10.2A8 8 0 1 1 4 12" />
        <path d="M4.2 6.4v3.8h3.8" />
        <path d="M12 8.2V12l2.8 1.8" />
      </g>
    </Icon>
  );
}

/** Olho: stealth ligado. */
export function EyeIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M2.5 12S6 6 12 6s9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6Z" />
        <circle cx="12" cy="12" r="2.6" />
      </g>
    </Icon>
  );
}

/** Olho cortado: stealth desligado. */
export function EyeOffIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M4 4.6l16 14.8" />
        <path d="M9.9 6.4A9.6 9.6 0 0 1 12 6c6 0 9.5 6 9.5 6a15 15 0 0 1-3.3 3.9" />
        <path d="M6.4 8.2A15.6 15.6 0 0 0 2.5 12S6 18 12 18a9.4 9.4 0 0 0 3.8-.8" />
        <path d="M9.6 10.1a2.9 2.9 0 0 0 4 3.9" />
      </g>
    </Icon>
  );
}

/** Seta para baixo: ocultar o overlay. */
export function ChevronDownIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <path
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M6 9.5l6 6 6-6"
      />
    </Icon>
  );
}

/** Alça de arraste (2×3 pontos). */
export function GripIcon({ className, size = 18 }: IconProps) {
  const dots: [number, number][] = [
    [9, 6],
    [15, 6],
    [9, 12],
    [15, 12],
    [9, 18],
    [15, 18],
  ];
  return (
    <Icon className={className} size={size}>
      <g fill="currentColor">
        {dots.map(([cx, cy]) => (
          <circle key={`${cx}-${cy}`} cx={cx} cy={cy} r="1.5" />
        ))}
      </g>
    </Icon>
  );
}

/** Globo: idioma da transcrição. */
export function LanguageIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      >
        <circle cx="12" cy="12" r="8.2" />
        <path d="M3.8 12h16.4" />
        <path d="M12 3.8c2.6 3.1 2.6 13.3 0 16.4" />
        <path d="M12 3.8c-2.6 3.1-2.6 13.3 0 16.4" />
      </g>
    </Icon>
  );
}

/** Microfone: canal de áudio ativo (mic/ambos). */
export function MicIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <rect x="9" y="3.2" width="6" height="10" rx="3" />
        <path d="M5.5 11.5a6.5 6.5 0 0 0 13 0" />
        <path d="M12 18v3" />
      </g>
    </Icon>
  );
}

/** Microfone cortado: captando apenas o áudio do sistema. */
export function MicOffIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M9 5a3 3 0 0 1 6 0v5" />
        <path d="M9 9.5V10a3 3 0 0 0 4.5 2.6" />
        <path d="M5.5 11.5a6.5 6.5 0 0 0 10.4 5.2" />
        <path d="M18.5 11.5a6.5 6.5 0 0 1-.6 2.7" />
        <path d="M12 18v3" />
        <path d="M4 4l16 16" />
      </g>
    </Icon>
  );
}

// A faísca da IA já existe em `TransportIcons` (usada nas bolhas); reexportada
// aqui para a barra/abas usarem um só desenho (BR-016/BR-008).
export { SparkIcon as SparkleIcon } from "./TransportIcons";

/** Dedo/toque: dica "sem clique" do drill-hover (BR-036). */
export function TouchIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M10 12V5.5a1.6 1.6 0 0 1 3.2 0V11" />
        <path d="M13.2 10.5V9a1.5 1.5 0 0 1 3 0v2" />
        <path d="M16.2 11.5V10a1.5 1.5 0 0 1 3 0v5.2c0 3-2 5.3-5.2 5.3h-1.2c-1.7 0-3.1-.8-4.1-2.3l-2.2-3.5a1.6 1.6 0 0 1 2.7-1.7L10 14.5" />
      </g>
    </Icon>
  );
}

/** Check simples: confirmar do drill-hover. */
export function CheckIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <path
        d="M5 12.5l4.2 4.2L19 7"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.9"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </Icon>
  );
}

/** Recolher/ocultar: duas setas diagonais apontando para o centro (task 13). */
export function CollapseIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M20 4l-7 7" />
        <path d="M13 11h3.5" />
        <path d="M13 11V7.5" />
        <path d="M4 20l7-7" />
        <path d="M11 13H7.5" />
        <path d="M11 13v3.5" />
      </g>
    </Icon>
  );
}
