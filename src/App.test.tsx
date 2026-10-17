import { describe, expect, it } from "vitest";
import { resolveRoute } from "./App";

describe("resolveRoute", () => {
  it("assume 'overlay' quando não está rodando dentro do Tauri", () => {
    expect(resolveRoute()).toBe("overlay");
  });
});
