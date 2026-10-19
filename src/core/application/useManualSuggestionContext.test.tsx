import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useManualSuggestionContext } from "./useManualSuggestionContext";

const manualSuggestionContext = vi.fn();
const clearManualSuggestionContext = vi.fn();
const subscribeMock = vi.hoisted(() => ({
  emit: null as null | ((payload: unknown) => void),
}));

vi.mock("../../adapters/ipc/suggestions", () => ({
  manualSuggestionContext: (...args: unknown[]) =>
    manualSuggestionContext(...args),
  clearManualSuggestionContext: (...args: unknown[]) =>
    clearManualSuggestionContext(...args),
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: (_event: string, handler: (payload: unknown) => void) => {
    subscribeMock.emit = handler;
    return Promise.resolve(() => {});
  },
}));

const context = {
  session_id: "s1",
  speaker: "THEY" as const,
  text: "Como tu faria um system design?",
  at_ms: 10_000,
};

describe("useManualSuggestionContext", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    manualSuggestionContext.mockResolvedValue(context);
    clearManualSuggestionContext.mockResolvedValue(undefined);
  });

  it("busca o contexto ao montar e a cada panel_opened", async () => {
    const { result } = renderHook(() =>
      useManualSuggestionContext({ sessionId: "s1" }),
    );

    await waitFor(() => expect(result.current.context?.text).toBe(context.text));
    expect(manualSuggestionContext).toHaveBeenCalledWith("s1");

    await act(async () => {
      subscribeMock.emit!("suggestions");
    });
    await waitFor(() =>
      expect(manualSuggestionContext).toHaveBeenCalledTimes(2),
    );
    expect(subscribeMock.emit).not.toBeNull();
  });

  it("limpa o contexto da sessão anterior ao trocar de sessão", async () => {
    const { rerender } = renderHook(
      (props: { sessionId: string }) =>
        useManualSuggestionContext({ sessionId: props.sessionId }),
      { initialProps: { sessionId: "s1" } },
    );
    await waitFor(() =>
      expect(manualSuggestionContext).toHaveBeenCalledWith("s1"),
    );

    rerender({ sessionId: "s2" });
    await waitFor(() =>
      expect(clearManualSuggestionContext).toHaveBeenCalledWith("s1"),
    );
    expect(manualSuggestionContext).toHaveBeenCalledWith("s2");
  });
});
