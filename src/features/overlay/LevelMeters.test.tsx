import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LevelMeters } from "./LevelMeters";
import type { AudioLevelEvent } from "../../core/domain/transcript";

const events = vi.hoisted(() => ({
  handlers: {} as Record<string, (payload: unknown) => void>,
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: async (name: string, handler: (payload: unknown) => void) => {
    events.handlers[name] = handler;
    return () => {};
  },
}));

const emitLevel = (payload: AudioLevelEvent) => {
  act(() => {
    events.handlers.audio_level?.(payload);
  });
};

const fillWidth = (label: string): string => {
  const row = screen.getByText(label).closest(".levels");
  const fill = row?.querySelector<HTMLElement>(".levels__fill");
  return fill?.style.width ?? "";
};

describe("LevelMeters", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    events.handlers = {};
  });

  it("assina audio_level e mostra o nível de cada canal", async () => {
    render(<LevelMeters channels={["microphone", "system"]} listening />);
    await waitFor(() => expect(events.handlers.audio_level).toBeDefined());

    emitLevel({ channel: "microphone", rms: 0.2, peak: 0.5 });
    emitLevel({ channel: "system", rms: 0.05, peak: 0.1 });

    // mic: max(rms×3, peak) = 0,6 · sistema: max(0,15; 0,1) = 0,15.
    expect(fillWidth("Microfone")).toBe("60%");
    expect(fillWidth("Sistema")).toBe("15%");
  });

  it("zera as barras quando a captura está parada", async () => {
    render(<LevelMeters channels={["microphone"]} listening={false} />);
    await waitFor(() => expect(events.handlers.audio_level).toBeDefined());

    emitLevel({ channel: "microphone", rms: 0.3, peak: 0.9 });

    expect(fillWidth("Microfone")).toBe("0%");
  });

  it("mostra 'Inativo' sem canais", () => {
    render(<LevelMeters channels={[]} listening />);
    expect(screen.getByText("Inativo")).toBeDefined();
  });
});
