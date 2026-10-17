import { describe, expect, it } from "vitest";
import { SHORTCUT_ACTIONS, defaultBindings } from "./shortcuts";

describe("shortcuts (domínio)", () => {
  it("padrões cobrem todas as ações", () => {
    const bindings = defaultBindings();
    expect(bindings).toHaveLength(SHORTCUT_ACTIONS.length);
    for (const action of SHORTCUT_ACTIONS) {
      expect(bindings.some((binding) => binding.action === action)).toBe(true);
    }
  });

  it("aceleradores padrão não são vazios", () => {
    for (const binding of defaultBindings()) {
      expect(binding.accelerator.length).toBeGreaterThan(0);
    }
  });
});
