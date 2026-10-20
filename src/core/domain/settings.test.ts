import { describe, expect, it } from "vitest";
import {
  ACCELERATION_LABELS,
  ACCELERATION_MODES,
  LANGUAGES,
  PROVIDERS,
  PROVIDER_DEFAULT_BASE_URLS,
  SOURCE_LANGUAGE_LABELS,
  TARGET_LANGUAGE_LABELS,
  WHISPER_MODELS,
  defaultSettings,
  providerRequiresBaseUrl,
} from "./settings";

describe("settings (domínio)", () => {
  it("defaults são válidos e coerentes", () => {
    const settings = defaultSettings();
    const first = settings.ai_sources[0];
    expect(settings.ai_sources.length).toBe(1);
    expect(first?.kind).toBe("openai");
    expect(first?.model).not.toBe("");
    expect(settings.translation_enabled).toBe(false);
  });

  it("a primeira fonte é a primária e o resto é fallback", () => {
    const settings = defaultSettings();
    expect(settings.ai_sources[0]?.enabled).toBe(true);
    expect(settings.ai_routes).toEqual([]);
  });

  it("o catálogo de providers inclui o DeepSeek", () => {
    expect(PROVIDERS).toContain("deepseek");
    // Não há catálogo de modelos: o usuário digita o nome.
    expect(PROVIDER_DEFAULT_BASE_URLS.deepseek).toBe(
      "https://api.deepseek.com/v1",
    );
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

  it("drill-hover começa ligado em 0,5 s", () => {
    const settings = defaultSettings();
    expect(settings.dwell_enabled).toBe(true);
    expect(settings.dwell_ms).toBe(500);
  });

  it("o agente Padrão é o selecionado por padrão (BR-059)", () => {
    const settings = defaultSettings();
    expect(settings.assistant_mode_id).toBe("default");
    // A migração do agente padrão ainda não rodou.
    expect(settings.assistant_mode_migrated).toBe(false);
  });
});
