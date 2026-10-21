import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useHudOpacity } from "./useHudOpacity";

const settingsIpc = vi.hoisted(() => ({
  HUD_OPACITY_PREVIEW_EVENT: "hud_opacity_preview",
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

describe("useHudOpacity", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventBus.handlers.clear();
    settingsIpc.getSettings.mockResolvedValue({ overlay_opacity: 0.6 });
    document.documentElement.style.removeProperty("--hud-alpha");
  });

  it("aplica a opacidade carregada no root do documento", async () => {
    renderHook(() => useHudOpacity());

    await waitFor(() =>
      expect(
        document.documentElement.style.getPropertyValue("--hud-alpha"),
      ).toBe("0.6"),
    );
  });

  it("aplica o preview de outra janela sem esperar o save", async () => {
    renderHook(() => useHudOpacity());
    await waitFor(() =>
      expect(
        document.documentElement.style.getPropertyValue("--hud-alpha"),
      ).toBe("0.6"),
    );
    await waitFor(() =>
      expect(eventBus.handlers.has("hud_opacity_preview")).toBe(true),
    );

    await act(async () => {
      eventBus.handlers.get("hud_opacity_preview")?.(0.3);
    });

    expect(
      document.documentElement.style.getPropertyValue("--hud-alpha"),
    ).toBe("0.3");
  });

  it("não deixa um settings_changed recente reverter o preview", async () => {
    renderHook(() => useHudOpacity());
    await waitFor(() =>
      expect(
        document.documentElement.style.getPropertyValue("--hud-alpha"),
      ).toBe("0.6"),
    );
    await waitFor(() =>
      expect(eventBus.handlers.has("settings_changed")).toBe(true),
    );

    await act(async () => {
      eventBus.handlers.get("hud_opacity_preview")?.(0.25);
      eventBus.handlers.get("settings_changed")?.(null);
    });

    expect(
      document.documentElement.style.getPropertyValue("--hud-alpha"),
    ).toBe("0.25");
  });
});
