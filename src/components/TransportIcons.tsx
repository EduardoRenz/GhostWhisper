/**
 * Ícones do produto (SVG desenhados no projeto).
 *
 * Não usamos a fonte Material Icons do Perssua: assets de terceiros não são
 * copiados (BR-008). As formas são as clássicas de transporte — triângulo (play),
 * duas barras (pause), quadrado (stop) — e uma faísca para a sugestão da IA.
 */

interface IconProps {
  className?: string;
}

/** Play/retomar. */
export function PlayIcon({ className }: IconProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      width="16"
      height="16"
      aria-hidden="true"
      focusable="false"
    >
      <path fill="currentColor" d="M8 5.2c0-.9 1-1.5 1.8-1L19 10.9c.7.4.7 1.4 0 1.8l-9.2 5.7c-.8.5-1.8-.1-1.8-1z" />
    </svg>
  );
}

/** Pausar. */
export function PauseIcon({ className }: IconProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      width="16"
      height="16"
      aria-hidden="true"
      focusable="false"
    >
      <rect fill="currentColor" x="7" y="5" width="3.6" height="14" rx="1.2" />
      <rect fill="currentColor" x="13.4" y="5" width="3.6" height="14" rx="1.2" />
    </svg>
  );
}

/** Parar/encerrar. */
export function StopIcon({ className }: IconProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      width="16"
      height="16"
      aria-hidden="true"
      focusable="false"
    >
      <rect fill="currentColor" x="6.5" y="6.5" width="11" height="11" rx="2" />
    </svg>
  );
}

/** Globo/idioma: botão de traduzir a bolha. */
export function TranslateIcon({ className }: IconProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      width="14"
      height="14"
      aria-hidden="true"
      focusable="false"
    >
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
    </svg>
  );
}

/** Faísca: identifica a sugestão da IA na conversa. */
export function SparkIcon({
  className,
  size = 12,
}: IconProps & { size?: number }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      width={size}
      height={size}
      aria-hidden="true"
      focusable="false"
    >
      <path
        fill="currentColor"
        d="M12 3.2l1.7 5.1 5.1 1.7-5.1 1.7L12 16.8l-1.7-5.1L5.2 10l5.1-1.7z"
      />
    </svg>
  );
}
