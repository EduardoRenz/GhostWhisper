/**
 * Setup comum dos testes do frontend.
 *
 * O idioma é estado global do módulo de l10n; cada teste começa em português
 * (idioma de origem) para não herdar uma troca feita pelo teste anterior.
 */

import { beforeEach } from "vitest";
import { setUiLanguage } from "./core/l10n";

/**
 * O jsdom 25 ainda não traz `PointerEvent`. Sem ele, `fireEvent.pointerDown`
 * cai para `Event` e perde `button`/`pointerId`, impedindo testar o botão
 * primário do arraste. O polyfill mínimo reaproveita `MouseEvent`.
 */
if (
  typeof window !== "undefined" &&
  typeof window.PointerEvent === "undefined"
) {
  class PointerEventPolyfill extends MouseEvent {
    readonly pointerId: number;
    readonly pointerType: string;
    readonly isPrimary: boolean;
    readonly width: number;
    readonly height: number;
    readonly pressure: number;

    constructor(type: string, init: PointerEventInit = {}) {
      super(type, init);
      this.pointerId = init.pointerId ?? 0;
      this.pointerType = init.pointerType ?? "mouse";
      this.isPrimary = init.isPrimary ?? true;
      this.width = init.width ?? 1;
      this.height = init.height ?? 1;
      this.pressure = init.pressure ?? 0;
    }
  }

  window.PointerEvent = PointerEventPolyfill as unknown as typeof PointerEvent;
}

beforeEach(() => {
  setUiLanguage("pt");
});
