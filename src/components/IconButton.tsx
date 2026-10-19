import type { ReactNode } from "react";

interface IconButtonProps {
  /** Ícone desenhado no projeto (BR-008). */
  children: ReactNode;
  /** Nome acessível — **obrigatório** (o botão não tem texto visível). */
  label: string;
  /** Dica de ferramenta; por padrão usa o `label`. */
  title?: string;
  onClick: () => void;
  /** Estado "ligado" (janela aberta, stealth ativo): realce por cor. */
  active?: boolean;
  /** Quando o botão é de alternância, expõe `aria-pressed`. */
  pressed?: boolean;
  disabled?: boolean;
  className?: string;
}

/**
 * Botão de ícone da barra (BR-032).
 *
 * Sem rótulo de texto visível: o acessível é garantido por `aria-label` (e
 * `title` para a dica do mouse). Só ele captura o mouse no overlay
 * (`data-interactive`); o resto do notch continua click-through.
 */
export function IconButton({
  children,
  label,
  title,
  onClick,
  active = false,
  pressed,
  disabled = false,
  className,
}: IconButtonProps) {
  const classes = [
    "icon-button",
    active ? "icon-button--on" : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      type="button"
      className={classes}
      data-interactive
      aria-label={label}
      aria-pressed={pressed === undefined ? undefined : pressed}
      title={title ?? label}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}
