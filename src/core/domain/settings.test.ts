import { describe, expect, it } from "vitest";
import {
  ACCELERATION_LABELS,
  ACCELERATION_MODES,
  LANGUAGES,
  PROVIDERS,
  SOURCE_LANGUAGE_LABELS,
  TARGET_LANGUAGE_LABELS,
  WHISPER_MODELS,
  defaultSettings,
  providerRequiresBaseUrl,
} from "./settings";

describe("settings (domínio)", () => {
  it("defaults são válidos e coerentes", () => {
    const settings = defaultSettings();
    expect(settings.model).not.toBe("");
    expect(settings.provider).toBe("openai");
    expect(settings.translation_enabled).toBe(false);
  });

  it("somente custom exige base_url", () => {
    expect(providerRequiresBaseUrl("custom")).toBe(true);
    for (const provider of PROVIDERS.filter((p) => p !== "custom")) {
      expect(providerRequiresBaseUrl(provider)).toBe(false);
    }
  });

  it("lista de modelos whisper não é vazia", () => {
    expect(WHISPER_MODELS.length).toBeGreaterThan(0);
  });

  it("aceleração padrão é auto com rótulos", () => {
    expect(defaultSettings().acceleration).toBe("auto");
    for (const mode of ACCELERATION_MODES) {
      expect(ACCELERATION_LABELS[mode]).toBeTruthy();
    }
  });

  it("idiomas suportados são auto/pt/en com rótulos", () => {
    expect(LANGUAGES).toEqual(["auto", "pt", "en"]);
    for (const language of LANGUAGES) {
      expect(SOURCE_LANGUAGE_LABELS[language]).toBeTruthy();
      expect(TARGET_LANGUAGE_LABELS[language]).toBeTruthy();
    }
    // Destino "auto" deixa claro que não traduz.
    expect(TARGET_LANGUAGE_LABELS.auto.toLowerCase()).toContain("traduz");
  });
});
