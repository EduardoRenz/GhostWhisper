import { describe, expect, it } from "vitest";
import {
  ACCELERATION_LABELS,
  ACCELERATION_MODES,
  DEFAULT_AI_FONT_SCALE,
  DEFAULT_FONT_SCALE,
  DEFAULT_OVERLAY_OPACITY,
  HUD_DENSITIES,
  HUD_DENSITY_LABELS,
  LANGUAGES,
  MAX_AI_FONT_SCALE,
  MAX_DWELL_MS,
  MAX_FONT_SCALE,
  MAX_OVERLAY_OPACITY,
  MIN_AI_FONT_SCALE,
  MIN_DWELL_MS,
  MIN_FONT_SCALE,
  MIN_OVERLAY_OPACITY,
  NOTCH_ANCHORS,
  NOTCH_ANCHOR_LABELS,
  PROVIDERS,
  PROVIDER_DEFAULT_BASE_URLS,
  SOURCE_LANGUAGE_LABELS,
  TARGET_LANGUAGE_LABELS,
  WHISPER_MODELS,
  clampAiFontScale,
  clampFontScale,
  clampOverlayOpacity,
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
    // Faixa do mockup (BR-062).
    expect(MIN_DWELL_MS).toBe(100);
    expect(MAX_DWELL_MS).toBe(1500);
  });

  it("a barra Perguntar nasce sem posição salva (BR-072)", () => {
    expect(defaultSettings().ask_position).toBeNull();
  });

  it("HUD começa equilibrado, no topo central e no padrão (BR-062)", () => {
    const settings = defaultSettings();
    expect(settings.overlay_opacity).toBe(0.75);
    expect(settings.notch_anchor).toBe("top_center");
    expect(settings.hud_density).toBe("standard");
    expect(NOTCH_ANCHORS).toEqual(["top_center", "top_right", "free"]);
    expect(HUD_DENSITIES).toEqual(["compact", "standard", "expanded"]);
    for (const anchor of NOTCH_ANCHORS) {
      expect(NOTCH_ANCHOR_LABELS[anchor]).toBeTruthy();
    }
    for (const density of HUD_DENSITIES) {
      expect(HUD_DENSITY_LABELS[density]).toBeTruthy();
    }
  });

  it("clampOverlayOpacity traz o valor para a faixa", () => {
    expect(clampOverlayOpacity(0.05)).toBe(MIN_OVERLAY_OPACITY);
    expect(clampOverlayOpacity(3)).toBe(MAX_OVERLAY_OPACITY);
    expect(clampOverlayOpacity(Number.NaN)).toBe(DEFAULT_OVERLAY_OPACITY);
    expect(clampOverlayOpacity(0.6)).toBe(0.6);
  });

  it("escalas de fonte nascem em 100 % e respeitam a faixa (BR-087)", () => {
    const settings = defaultSettings();
    expect(settings.font_scale).toBe(1);
    expect(settings.ai_font_scale).toBe(1);
    expect(DEFAULT_FONT_SCALE).toBe(1);
    expect(MIN_FONT_SCALE).toBe(0.75);
    expect(MAX_FONT_SCALE).toBe(1.6);
    // O bônus de IA só acrescenta à escala global.
    expect(MIN_AI_FONT_SCALE).toBe(1);
    expect(MAX_AI_FONT_SCALE).toBe(1.5);

    expect(clampFontScale(0.1)).toBe(MIN_FONT_SCALE);
    expect(clampFontScale(4)).toBe(MAX_FONT_SCALE);
    expect(clampFontScale(Number.NaN)).toBe(DEFAULT_FONT_SCALE);
    expect(clampFontScale(1.25)).toBe(1.25);

    expect(clampAiFontScale(0.4)).toBe(MIN_AI_FONT_SCALE);
    expect(clampAiFontScale(9)).toBe(MAX_AI_FONT_SCALE);
    expect(clampAiFontScale(Number.NaN)).toBe(DEFAULT_AI_FONT_SCALE);
    expect(clampAiFontScale(1.2)).toBe(1.2);
  });

  it("as perguntas prontas terminam com o pedido da última pergunta (BR-054)", () => {
    const settings = defaultSettings();
    expect(settings.saved_questions.map((question) => question.text)).toEqual([
      "Resumir tudo até agora",
      "Listar as ações",
      "Listar as principais ideias",
      "O que posso falar?",
      "Responda a última pergunta identificada",
    ]);
    // A migração do preset ainda não rodou nesta instalação nova.
    expect(settings.saved_questions_migrated).toBe(false);
  });

  it("o agente Padrão é o selecionado por padrão (BR-059)", () => {
    const settings = defaultSettings();
    expect(settings.assistant_mode_id).toBe("default");
    // A migração do agente padrão ainda não rodou.
    expect(settings.assistant_mode_migrated).toBe(false);
  });
});
