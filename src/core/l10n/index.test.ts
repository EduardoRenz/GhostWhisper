import { describe, expect, it, vi } from "vitest";
import {
  EN,
  PT,
  TRANSLATION_KEYS,
  normalizeUiLanguage,
  setUiLanguage,
  subscribeUiLanguage,
  translate,
  uiLanguageFromLocale,
} from "./index";

describe("l10n — catálogo e resolução de idioma", () => {
  it("pt e en têm o mesmo conjunto de chaves, sem texto vazio", () => {
    expect(Object.keys(EN).sort()).toEqual(Object.keys(PT).sort());
    for (const key of TRANSLATION_KEYS) {
      expect(PT[key].trim(), `pt vazio: ${key}`).not.toBe("");
      expect(EN[key].trim(), `en vazio: ${key}`).not.toBe("");
    }
  });

  it("os placeholders de pt e en coincidem", () => {
    const placeholders = (text: string) =>
      [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();

    for (const key of TRANSLATION_KEYS) {
      expect(placeholders(EN[key]), `placeholders de ${key}`).toEqual(
        placeholders(PT[key]),
      );
    }
  });

  it("traduz pela chave e interpola parâmetros", () => {
    expect(translate("settings_tab_general", "pt")).toBe("Geral & HUD");
    expect(translate("settings_tab_general", "en")).toBe("General & HUD");
    expect(
      translate("assistants_configured_agents_count", "en", { count: 3 }),
    ).toBe("Configured agents (3)");
  });

  it("devolve a própria chave quando ela não existe no catálogo", () => {
    // Cast deliberado: simula uma chave inválida em runtime.
    const key = "chave_inexistente" as never;
    expect(translate(key, "en")).toBe("chave_inexistente");
  });

  it("normaliza o código persistido", () => {
    expect(normalizeUiLanguage(" PT ")).toBe("pt");
    expect(normalizeUiLanguage("en")).toBe("en");
    expect(normalizeUiLanguage("es")).toBeNull();
    expect(normalizeUiLanguage(null)).toBeNull();
  });

  it("resolve o locale do sistema na primeira inicialização", () => {
    expect(uiLanguageFromLocale("pt_BR.UTF-8")).toBe("pt");
    expect(uiLanguageFromLocale("pt-PT")).toBe("pt");
    expect(uiLanguageFromLocale("en_US.UTF-8")).toBe("en");
    expect(uiLanguageFromLocale("es_ES.UTF-8")).toBe("en");
    expect(uiLanguageFromLocale(null)).toBe("en");
  });

  it("notifica os assinantes quando o idioma muda", () => {
    const listener = vi.fn();
    const unsubscribe = subscribeUiLanguage(listener);
    setUiLanguage("en");
    expect(listener).toHaveBeenCalledTimes(1);
    setUiLanguage("en");
    expect(listener).toHaveBeenCalledTimes(1);
    unsubscribe();
  });
});
