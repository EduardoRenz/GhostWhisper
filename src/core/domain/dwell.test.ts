import { describe, expect, it } from "vitest";
import { clampDwellMs, dwellLabel } from "./dwell";
import {
  DEFAULT_DWELL_MS,
  MAX_DWELL_MS,
  MIN_DWELL_MS,
} from "./settings";

describe("dwell (domínio)", () => {
  it("faz clamp na faixa e arredonda", () => {
    expect(clampDwellMs(1)).toBe(MIN_DWELL_MS);
    expect(clampDwellMs(9_999)).toBe(MAX_DWELL_MS);
    expect(clampDwellMs(499.6)).toBe(500);
    expect(clampDwellMs(Number.NaN)).toBe(DEFAULT_DWELL_MS);
  });

  it("rotula a duração em segundos pt-BR", () => {
    expect(dwellLabel(500)).toBe("0,5 s");
    expect(dwellLabel(1_200)).toBe("1,2 s");
    // Acima do teto (1,5 s desde a BR-062) o rótulo reflete o clamp.
    expect(dwellLabel(2_000)).toBe("1,5 s");
  });
});
