/**
 * Posicionamento de tooltips (BR-081).
 *
 * Lógica **pura** (sem DOM): recebe os retângulos do gatilho, do balão e da
 * janela e decide o lado com espaço, centralizando e prendendo o balão dentro
 * da viewport. Assim o tooltip não fica oculto por containers com `overflow`
 * (o balão é renderizado em portal) nem escapa da janela.
 */

/** Retângulo em px de viewport. */
export interface TooltipRect {
  top: number;
  left: number;
  width: number;
  height: number;
}

/** Tamanho em px. */
export interface TooltipSize {
  width: number;
  height: number;
}

/** Resultado: posição `fixed` e o lado escolhido. */
export interface TooltipPlacement {
  top: number;
  left: number;
  placement: "top" | "bottom";
}

/** Folga mínima entre o balão e a borda da janela. */
export const TOOLTIP_MARGIN = 8;

/** Espaço entre o gatilho e o balão. */
export const TOOLTIP_GAP = 6;

/** `true` quando o balão cabe abaixo do gatilho com folga. */
function fitsBelow(
  anchor: TooltipRect,
  size: TooltipSize,
  viewport: TooltipSize,
): boolean {
  const spaceBelow = viewport.height - (anchor.top + anchor.height);
  return spaceBelow >= size.height + TOOLTIP_GAP + TOOLTIP_MARGIN;
}

/** Prende `value` entre `min` e `max` (quando `max < min`, vale `min`). */
function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), Math.max(min, max));
}

/**
 * Calcula a posição do balão.
 *
 * - Prefere **abaixo** do gatilho; sem espaço, abre **acima** (se houver mais
 *   espaço acima do que abaixo); em janelas muito baixas, prende na viewport.
 * - Centraliza no gatilho e prende nas bordas laterais.
 */
export function tooltipPosition(
  anchor: TooltipRect,
  size: TooltipSize,
  viewport: TooltipSize,
): TooltipPlacement {
  const spaceAbove = anchor.top;
  const below = fitsBelow(anchor, size, viewport);
  const spaceBelow = viewport.height - (anchor.top + anchor.height);
  const placement: "top" | "bottom" =
    below || spaceBelow >= spaceAbove ? "bottom" : "top";

  const rawTop =
    placement === "bottom"
      ? anchor.top + anchor.height + TOOLTIP_GAP
      : anchor.top - TOOLTIP_GAP - size.height;
  const top = clamp(
    rawTop,
    TOOLTIP_MARGIN,
    viewport.height - size.height - TOOLTIP_MARGIN,
  );

  const rawLeft = anchor.left + anchor.width / 2 - size.width / 2;
  const left = clamp(
    rawLeft,
    TOOLTIP_MARGIN,
    viewport.width - size.width - TOOLTIP_MARGIN,
  );

  return { top, left, placement };
}
