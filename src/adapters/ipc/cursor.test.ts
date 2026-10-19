import { describe, expect, it } from "vitest";
import { cursorSampleFrom } from "./cursor";

describe("cursorSampleFrom", () => {
  const origin = { x: 100, y: 100 };
  const size = { width: 260, height: 60 };

  it("detecta o cursor dentro e fora da janela", () => {
    expect(
      cursorSampleFrom({ x: 150, y: 130 }, origin, size, 1, null).inside,
    ).toBe(true);
    expect(
      cursorSampleFrom({ x: 99, y: 130 }, origin, size, 1, null).inside,
    ).toBe(false);
    expect(
      cursorSampleFrom({ x: 150, y: 160 }, origin, size, 1, null).inside,
    ).toBe(false);
  });

  it("detecta o cursor sobre o botão Sim", () => {
    const confirm = { left: 120, top: 10, right: 240, bottom: 50 };
    expect(
      cursorSampleFrom({ x: 250, y: 125 }, origin, size, 1, confirm),
    ).toEqual({ inside: true, overConfirm: true });
    expect(
      cursorSampleFrom({ x: 150, y: 125 }, origin, size, 1, confirm),
    ).toEqual({ inside: true, overConfirm: false });
  });

  it("converte px físicos com scale 2", () => {
    // Janela física em (100,100), 520×120, scale 2 → CSS 260×60.
    expect(
      cursorSampleFrom(
        { x: 300, y: 200 },
        origin,
        { width: 520, height: 120 },
        2,
        null,
      ).inside,
    ).toBe(true);
    expect(
      cursorSampleFrom(
        { x: 300, y: 340 },
        origin,
        { width: 520, height: 120 },
        2,
        null,
      ).inside,
    ).toBe(false);
  });
});
