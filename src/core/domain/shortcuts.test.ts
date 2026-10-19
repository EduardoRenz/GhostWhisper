import { describe, expect, it } from "vitest";
import {
  SHORTCUT_ACTIONS,
  acceleratorBadges,
  defaultBindings,
  parseAccelerator,
} from "./shortcuts";

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

  it("parseAccelerator quebra e limpa os tokens", () => {
    expect(parseAccelerator("CommandOrControl+Shift+S")).toEqual([
      "CommandOrControl",
      "Shift",
      "S",
    ]);
    expect(parseAccelerator("Ctrl + + A")).toEqual(["Ctrl", "A"]);
    expect(parseAccelerator("")).toEqual([]);
  });

  it("acceleratorBadges traduz modificadores e teclas", () => {
    expect(acceleratorBadges("CommandOrControl+Shift+S")).toEqual([
      "Ctrl",
      "Shift",
      "S",
    ]);
    // Variantes de modificador caem no mesmo rótulo.
    expect(acceleratorBadges("Cmd+Alt+P")).toEqual(["Ctrl", "Alt", "P"]);
    expect(acceleratorBadges("Control+Option+F1")).toEqual([
      "Ctrl",
      "Alt",
      "F1",
    ]);
    expect(acceleratorBadges("Super+Enter")).toEqual(["Super", "Enter"]);
  });

  it("acceleratorBadges devolve [] quando não há atalho", () => {
    expect(acceleratorBadges("")).toEqual([]);
    expect(acceleratorBadges("   ")).toEqual([]);
  });
});
