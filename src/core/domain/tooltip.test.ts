import { describe, expect, it } from "vitest";
import {
  TOOLTIP_GAP,
  TOOLTIP_MARGIN,
  tooltipPosition,
} from "./tooltip";

const viewport = { width: 800, height: 600 };
const size = { width: 200, height: 80 };

describe("tooltipPosition", () => {
  it("abre abaixo quando há espaço", () => {
    const anchor = { top: 100, left: 300, width: 20, height: 20 };
    const position = tooltipPosition(anchor, size, viewport);

    expect(position.placement).toBe("bottom");
    expect(position.top).toBe(anchor.top + anchor.height + TOOLTIP_GAP);
    expect(position.left).toBe(210); // centralizado: 300 + 10 - 100
  });

  it("abre acima quando não há espaço abaixo", () => {
    const anchor = { top: 560, left: 300, width: 20, height: 20 };
    const position = tooltipPosition(anchor, size, viewport);

    expect(position.placement).toBe("top");
    expect(position.top).toBe(anchor.top - TOOLTIP_GAP - size.height);
  });

  it("prende no topo quando nem acima nem abaixo cabem", () => {
    const tiny = { width: 300, height: 60 };
    const anchor = { top: 20, left: 100, width: 20, height: 20 };
    const position = tooltipPosition(anchor, size, tiny);

    expect(position.top).toBe(TOOLTIP_MARGIN);
  });

  it("prende na borda direita da janela", () => {
    const anchor = { top: 100, left: 780, width: 20, height: 20 };
    const position = tooltipPosition(anchor, size, viewport);

    expect(position.left).toBe(viewport.width - size.width - TOOLTIP_MARGIN);
  });

  it("prende na borda esquerda da janela", () => {
    const anchor = { top: 100, left: 0, width: 20, height: 20 };
    const position = tooltipPosition(anchor, size, viewport);

    expect(position.left).toBe(TOOLTIP_MARGIN);
  });

  it("nunca devolve posição negativa em janela menor que o balão", () => {
    const tiny = { width: 120, height: 50 };
    const anchor = { top: 10, left: 10, width: 20, height: 20 };
    const position = tooltipPosition(anchor, size, tiny);

    expect(position.top).toBe(TOOLTIP_MARGIN);
    expect(position.left).toBe(TOOLTIP_MARGIN);
  });
});
