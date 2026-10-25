/**
 * Regra pura do arraste pelo header.
 *
 * O header inteiro inicia o arraste da janela (BR-064), mas os controles
 * precisam continuar recebendo o clique. A alça `.drag-handle` também entra na
 * lista: ela tem handler próprio e o header não deve disparar um segundo
 * `startDragging`.
 */
const NON_DRAG_SELECTOR = [
  "button",
  "a[href]",
  "input",
  "select",
  "textarea",
  "label",
  "summary",
  "[role='button']",
  "[role='link']",
  "[role='switch']",
  "[role='slider']",
  "[role='tab']",
  "[contenteditable='true']",
  "[data-no-window-drag]",
  ".drag-handle",
].join(",");

/**
 * Diz se o alvo do `pointerdown` pode iniciar o arraste do header.
 *
 * `true` para áreas inertes (título, subtítulo, selo, espaços vazios);
 * `false` para controles e para a alça de arraste.
 */
export function shouldStartHeaderDrag(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) {
    return false;
  }
  return target.closest(NON_DRAG_SELECTOR) === null;
}
