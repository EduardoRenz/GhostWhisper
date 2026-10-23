import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { applyFontScale, useFontScale } from "./useFontScale";

const settingsIpc = vi.hoisted(() => ({
  FONT_SCALES_PREVIEW_EVENT: "font_scales_preview",
  getSettings: vi.fn(),
}));

const eventBus = vi.hoisted(() => {
  const handlers = new Map<string, (payload: unknown) => void>();
  return {
    handlers,
    subscribe: vi.fn(
      async (event: string, handler: (payload: unknown) => void) => {
        handlers.set(event, handler);
        return () => {};
      },
    ),
  };
});

vi.mock("../../adapters/ipc/settings", () => settingsIpc);
vi.mock("../../adapters/ipc/events", () => ({
  subscribe: eventBus.subscribe,
}));

const root = () => document.documentElement;
const inlineFontSize = () => root().style.getPropertyValue("font-size");
const aiScale = () => root().style.getPropertyValue("--oh-ai-font-scale");

/**
 * Verifica o zoom aplicado no `<html>`.
 *
 * O jsdom normaliza `calc(16px * 1.4)` para `calc(22.4px)`; o navegador
 * (WebKitGTK) preserva a forma autorada — o teste aceita as duas.
 */
function expectZoom(value: string, scale: number): void {
  const candidates = new Set([
    `calc(16px * ${scale})`,
    `calc(${Number((16 * scale).toFixed(4))}px)`,
    `calc(${16 * scale}px)`,
  ]);
  expect(candidates.has(value), `font-size era "${value}"`).toBe(true);
}

describe("useFontScale", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventBus.handlers.clear();
    settingsIpc.getSettings.mockResolvedValue({
      font_scale: 1.4,
      ai_font_scale: 1.2,
    });
    root().style.removeProperty("font-size");
    root().style.removeProperty("--oh-ai-font-scale");
  });

  it("aplica as escalas carregadas no root do documento", async () => {
    renderHook(() => useFontScale());

    await waitFor(() => expect(aiScale()).toBe("1.2"));
    expectZoom(inlineFontSize(), 1.4);
  });

  it("remove o zoom ao voltar para 100 % (nada muda no tamanho base)", async () => {
    renderHook(() => useFontScale());
    await waitFor(() => expect(inlineFontSize()).not.toBe(""));

    await act(async () => {
      settingsIpc.getSettings.mockResolvedValue({
        font_scale: 1,
        ai_font_scale: 1,
      });
      eventBus.handlers.get("settings_changed")?.(null);
    });

    await waitFor(() => expect(inlineFontSize()).toBe(""));
    expect(aiScale()).toBe("1");
  });

  it("aplica o preview de outra janela sem esperar o save", async () => {
    renderHook(() => useFontScale());
    await waitFor(() =>
      expect(eventBus.handlers.has("font_scales_preview")).toBe(true),
    );

    await act(async () => {
      eventBus.handlers.get("font_scales_preview")?.({
        font_scale: 1.6,
        ai_font_scale: 1.5,
      });
    });

    expectZoom(inlineFontSize(), 1.6);
    expect(aiScale()).toBe("1.5");
  });

  it("não deixa um settings_changed recente reverter o preview", async () => {
    renderHook(() => useFontScale());
    await waitFor(() =>
      expect(eventBus.handlers.has("settings_changed")).toBe(true),
    );

    await act(async () => {
      eventBus.handlers.get("font_scales_preview")?.({
        font_scale: 0.8,
        ai_font_scale: 1.3,
      });
      eventBus.handlers.get("settings_changed")?.(null);
    });

    expectZoom(inlineFontSize(), 0.8);
    expect(aiScale()).toBe("1.3");
  });

  it("fora das janelas de conteúdo não aplica nada (notch e askbar)", async () => {
    settingsIpc.getSettings.mockResolvedValue({
      font_scale: 1.4,
      ai_font_scale: 1.2,
    });
    renderHook(() => useFontScale(false));

    await waitFor(() =>
      expect(eventBus.handlers.has("font_scales_preview")).toBe(true),
    );
    await act(async () => {
      eventBus.handlers.get("font_scales_preview")?.({
        font_scale: 1.4,
        ai_font_scale: 1.2,
      });
    });

    expect(inlineFontSize()).toBe("");
    expect(aiScale()).toBe("1");
  });

  it("trata valores fora da faixa como o limite mais próximo", async () => {
    renderHook(() => useFontScale());
    await waitFor(() =>
      expect(eventBus.handlers.has("font_scales_preview")).toBe(true),
    );

    await act(async () => {
      eventBus.handlers.get("font_scales_preview")?.({
        font_scale: 9,
        ai_font_scale: 0.1,
      });
    });

    expectZoom(inlineFontSize(), 1.6);
    expect(aiScale()).toBe("1");
  });
});

describe("applyFontScale", () => {
  beforeEach(() => {
    root().style.removeProperty("font-size");
    root().style.removeProperty("--oh-ai-font-scale");
  });

  it("mantém a base do sistema em 100 % e ignora valores não finitos", () => {
    applyFontScale(1, 1, true);
    expect(inlineFontSize()).toBe("");
    expect(aiScale()).toBe("1");

    applyFontScale(Number.NaN, Number.NaN, true);
    expect(inlineFontSize()).toBe("");
    expect(aiScale()).toBe("1");
  });

  it("sempre desliga o bônus de IA quando a janela está fora do escopo", () => {
    applyFontScale(1.2, 1.5, false);
    expect(inlineFontSize()).toBe("");
    expect(aiScale()).toBe("1");
  });
});
