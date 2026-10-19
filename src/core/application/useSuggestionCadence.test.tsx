import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useSuggestionCadence } from "./useSuggestionCadence";

const subscribeMock = vi.hoisted(() => ({
  emit: null as null | ((payload: unknown) => void),
  events: [] as string[],
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: (event: string, handler: (payload: unknown) => void) => {
    subscribeMock.events.push(event);
    subscribeMock.emit = handler;
    return Promise.resolve(() => {});
  },
}));

describe("useSuggestionCadence", () => {
  beforeEach(() => {
    subscribeMock.emit = null;
    subscribeMock.events = [];
  });

  it("assina o evento do core ao montar (sem depender de `opened`)", async () => {
    // O controle já ficou parado em 0 s quando a assinatura dependia da
    // visibilidade da janela; a assinatura é incondicional.
    renderHook(() => useSuggestionCadence());
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());
    expect(subscribeMock.events).toContain("suggestion_cadence");
  });

  it("atualiza a contagem quando o core publica", async () => {
    const { result } = renderHook(() => useSuggestionCadence());
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!({
        enabled: true,
        running: true,
        remaining_ms: 12_000,
        awaiting_speech: false,
        window_ms: 20_000,
      });
    });

    expect(result.current.remaining_ms).toBe(12_000);
    expect(result.current.awaiting_speech).toBe(false);
    expect(result.current.enabled).toBe(true);
  });

  it("reflete o estado de espera por fala", async () => {
    const { result } = renderHook(() => useSuggestionCadence());
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!({
        enabled: true,
        running: true,
        remaining_ms: 0,
        awaiting_speech: true,
        window_ms: 20_000,
      });
    });

    expect(result.current.awaiting_speech).toBe(true);
    expect(result.current.remaining_ms).toBe(0);
  });
});
