/**
 * Posição do cursor em relação à janela de confirmação.
 *
 * Usado pelo drill-hover: depois de esconder/mostrar a janela, o webview pode
 * manter um estado de hover obsoleto e não reemitir `pointerenter`. A posição
 * global do cursor (com a geometria enviada pelo core) é a fonte confiável para
 * pausar o cancelamento e iniciar o segundo dwell.
 */
import { cursorPosition } from "@tauri-apps/api/window";
import { isTauriRuntime } from "./client";

export interface DwellCursorSample {
  /** Cursor dentro dos limites da janela (área de confirmação). */
  inside: boolean;
  /** Cursor sobre o botão "Sim". */
  overConfirm: boolean;
}

/**
 * Geometria física da janela `dwell`, enviada pelo core no `dwell_context`.
 *
 * Vem do adaptador (posição/tamanho reais após `show`), não do webview — evita
 * corrida de posição na primeira amostra e não exige permissões de janela.
 */
export interface DwellWindowGeometry {
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
}

/**
 * Amostra o cursor e devolve se está na janela e sobre o "Sim".
 *
 * `undefined` fora do Tauri ou se a posição não estiver disponível. As
 * coordenadas são convertidas para CSS px relativos ao conteúdo da janela.
 */
export async function sampleDwellCursor(
  confirm: Element | null,
  geometry: DwellWindowGeometry,
): Promise<DwellCursorSample | undefined> {
  if (!isTauriRuntime()) {
    return undefined;
  }
  try {
    const cursor = await cursorPosition();
    const rect = confirm?.getBoundingClientRect();
    return cursorSampleFrom(
      cursor,
      { x: geometry.x, y: geometry.y },
      { width: geometry.width, height: geometry.height },
      geometry.scale,
      rect
        ? {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
          }
        : null,
    );
  } catch {
    return undefined;
  }
}

/** Matemática pura (testável): cursor físico → dentro da janela/confirmar. */
export function cursorSampleFrom(
  cursor: { x: number; y: number },
  origin: { x: number; y: number },
  size: { width: number; height: number },
  scale: number,
  confirm: { left: number; top: number; right: number; bottom: number } | null,
): DwellCursorSample {
  if (size.width <= 0 || size.height <= 0 || scale <= 0) {
    return { inside: false, overConfirm: false };
  }
  const x = (cursor.x - origin.x) / scale;
  const y = (cursor.y - origin.y) / scale;
  const inside =
    x >= 0 && y >= 0 && x < size.width / scale && y < size.height / scale;
  const overConfirm =
    inside &&
    !!confirm &&
    x >= confirm.left &&
    x <= confirm.right &&
    y >= confirm.top &&
    y <= confirm.bottom;
  return { inside, overConfirm };
}
