import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useSessionState } from "./useSessionState";

const sessionCurrent = vi.fn();
const subscribe = vi.fn();

vi.mock("../../adapters/ipc/session", () => ({
  sessionStart: vi.fn(),
  sessionStop: vi.fn(),
  sessionPause: vi.fn(),
  sessionResume: vi.fn(),
  sessionSetChannels: vi.fn(),
  sessionContinue: vi.fn(),
  sessionCurrent: (...args: unknown[]) => sessionCurrent(...args),
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: (...args: unknown[]) => subscribe(...args),
}));

describe("useSessionState", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    subscribe.mockResolvedValue(() => {});
  });

  it("semeia a sessão ativa ao abrir no meio da sessão", async () => {
    sessionCurrent.mockResolvedValue("s1");

    const { result } = renderHook(() => useSessionState());

    await waitFor(() => {
      expect(result.current.state.session_id).toBe("s1");
    });
    expect(result.current.state.listening).toBe(true);
  });

  it("não sobrescreve o estado de um evento já recebido", async () => {
    sessionCurrent.mockResolvedValue("seed");
    subscribe.mockImplementation((event: string, handler: (payload: unknown) => void) => {
      if (event === "session_state") {
        handler({
          listening: true,
          paused: true,
          session_id: "s2",
          channels: [],
          started_at_ms: 1,
        });
      }
      return Promise.resolve(() => {});
    });

    const { result } = renderHook(() => useSessionState());

    await waitFor(() => {
      expect(result.current.state.session_id).toBe("s2");
    });
    expect(result.current.state.paused).toBe(true);
  });
});
