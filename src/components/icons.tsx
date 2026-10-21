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

/** Barras de áudio: selo da aba "Transcrição ao Vivo" (BR-040). */
export function WaveformIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
      >
        <path d="M4 10.5v3" />
        <path d="M8 7.5v9" />
        <path d="M12 5v14" />
        <path d="M16 8.5v7" />
        <path d="M20 10.5v3" />
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

/** Lupa: o botão "Perguntar" (BR-057). */
export function SearchIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <circle cx="10.6" cy="10.6" r="6.2" />
        <path d="M15.4 15.4 20.4 20.4" />
      </g>
    </Icon>
  );
}

/** Diário: a aba "Sessão" (BR-052). */
export function JournalIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M6 3.5h12v17H6Z" />
        <path d="M9 8h6" />
        <path d="M9 12h6" />
        <path d="M9 16h3.5" />
      </g>
    </Icon>
  );
}

/** Câmera: o print de tela (BR-055). */
export function CameraIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M3.6 8.4h3.2l1.4-2.2h7.6l1.4 2.2h3.2v10.4H3.6Z" />
        <circle cx="12" cy="13.4" r="3.4" />
      </g>
    </Icon>
  );
}

/** Câmera de vídeo: gravar a sessão (BR-065), diferente do print de tela. */
export function VideoCameraIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <rect x="2.6" y="7" width="12.4" height="10" rx="2.2" />
        <path d="m15 10.6 6-2.9v8.6l-6-2.9" />
      </g>
    </Icon>
  );
}

/** X: remover anexo. */
export function CloseIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
      >
        <path d="M6 6l12 12" />
        <path d="M18 6 6 18" />
      </g>
    </Icon>
  );
}

/** Avião de papel: enviar a pergunta. */
export function SendIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M20.5 3.5 10.8 13.2" />
        <path d="M20.5 3.5 14 20.5l-3.2-7.3L3.5 10Z" />
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

/** Controles deslizantes: contexto extra. */
export function SlidersIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M4 7.2h9.4" />
        <path d="M17 7.2h3" />
        <path d="M4 16.8h3" />
        <path d="M10.6 16.8h9.4" />
        <circle cx="15.2" cy="7.2" r="2.2" />
        <circle cx="8.8" cy="16.8" r="2.2" />
      </g>
    </Icon>
  );
}

/** Copiar: dois retângulos sobrepostos. */
export function CopyIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <rect x="8.6" y="8.6" width="11" height="11" rx="2" />
        <path d="M15.4 5.6H6.4a1.8 1.8 0 0 0-1.8 1.8v9" />
      </g>
    </Icon>
  );
}

/** Fixar: um alfinete. */
export function PinIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M9.4 3.6h5.2l-.8 5.1 3 3.1H6.2l3-3.1z" />
        <path d="M12 11.8v8.6" />
      </g>
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

/** Pessoas: a aba "Assistente" (agentes). */
export function UsersIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <circle cx="9.2" cy="8.4" r="3.1" />
        <path d="M3.6 19.4c.5-3 2.8-4.8 5.6-4.8s5.1 1.8 5.6 4.8" />
        <path d="M16.2 5.9a3 3 0 0 1 0 5.6" />
        <path d="M17.6 14.9c1.7.6 2.9 2 3.2 4.1" />
      </g>
    </Icon>
  );
}

/** Teclado: a aba "Atalhos". */
export function KeyboardIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <rect x="2.8" y="6" width="18.4" height="12" rx="2.2" />
        <path d="M6.4 9.6h.01M9.6 9.6h.01M12.8 9.6h.01M16 9.6h.01M8 14.4h8" />
        <path d="M18.8 12.2h.01M6.4 12.2h.01M9.6 12.2h.01M12.8 12.2h.01M16 12.2h.01" />
      </g>
    </Icon>
  );
}

/** Raio: a aba "Fontes de IA" (energia da IA). */
export function BoltIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M13.2 2.8 5.4 13.4h5.2l-.8 7.8 7.8-10.6h-5.2z" />
      </g>
    </Icon>
  );
}

/** Pasta: abrir o local do arquivo. */
export function FolderIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <path
        d="M3.4 6.6h6l1.8 2.2h9.4v9.4a1.4 1.4 0 0 1-1.4 1.4H4.8a1.4 1.4 0 0 1-1.4-1.4Z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinejoin="round"
      />
    </Icon>
  );
}

/** Seta para baixo: baixar a sessão. */
export function DownloadIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M12 4v10.4" />
        <path d="m7.8 10.6 4.2 4.2 4.2-4.2" />
        <path d="M4.6 19.4h14.8" />
      </g>
    </Icon>
  );
}

/** Lixeira: descartar a gravação. */
export function TrashIcon({ className, size = 18 }: IconProps) {
  return (
    <Icon className={className} size={size}>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M4.6 7.2h14.8" />
        <path d="M9.2 7.2V5.4h5.6v1.8" />
        <path d="M6.4 7.2 7.3 20h9.4l.9-12.8" />
        <path d="M10.2 11v5.4M13.8 11v5.4" />
      </g>
    </Icon>
  );
}
