import { describe, expect, it } from "vitest";
import {
  WAVEFORM_BARS,
  waveformHeight,
  waveformLevel,
} from "./waveform";

describe("waveform (domínio)", () => {
  it("ignora silêncio e valores inválidos", () => {
    expect(waveformLevel(0)).toBe(0);
    expect(waveformLevel(-0.2)).toBe(0);
    expect(waveformLevel(Number.NaN)).toBe(0);
    expect(waveformLevel(Number.POSITIVE_INFINITY)).toBe(0);
  });

  it("amplifica fala baixa e satura em 1", () => {
    expect(waveformLevel(0.01)).toBeGreaterThan(0.1);
    expect(waveformLevel(0.01)).toBeLessThan(1);
    expect(waveformLevel(1)).toBe(1);
    expect(waveformLevel(10)).toBe(1);
  });

  it("mantém a altura entre 4 e 14 px", () => {
    expect(waveformHeight(0)).toBe(4);
    expect(waveformHeight(0.5)).toBe(9);
    expect(waveformHeight(1)).toBe(14);
    expect(waveformHeight(2)).toBe(14);
  });

  it("expõe 4 barras", () => {
    expect(WAVEFORM_BARS).toBe(4);
  });
});
