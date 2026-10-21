import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTranscriptActivity } from "./useTranscriptActivity";

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

describe("useTranscriptActivity", () => {
  beforeEach(() => {
    subscribeMock.emit = null;
    subscribeMock.events = [];
  });

  it("assina o evento do core para a sessão atual", async () => {
    renderHook(() => useTranscriptActivity("s1"));
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());
    expect(subscribeMock.events).toContain("transcript_activity");
  });

  it("liga microfone e Outros por canal", async () => {
    const { result } = renderHook(() => useTranscriptActivity("s1"));
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!({
        session_id: "s1",
        channel: "microphone",
        processing: true,
      });
    });
    expect(result.current).toEqual({ microphone: true, system: false });

    act(() => {
      subscribeMock.emit!({
        session_id: "s1",
        channel: "system",
        processing: true,
      });
    });
    expect(result.current).toEqual({ microphone: true, system: true });

    act(() => {
      subscribeMock.emit!({
        session_id: "s1",
        channel: "microphone",
        processing: false,
      });
    });
    expect(result.current).toEqual({ microphone: false, system: true });
  });

  it("ignora eventos de outra sessão e zera ao trocar de sessão", async () => {
    const { result, rerender } = renderHook(
      ({ session }: { session: string | null }) => useTranscriptActivity(session),
      { initialProps: { session: "s1" } },
    );
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!({
        session_id: "s2",
        channel: "microphone",
        processing: true,
      });
    });
    expect(result.current.microphone).toBe(false);

    act(() => {
      subscribeMock.emit!({
        session_id: "s1",
        channel: "system",
        processing: true,
      });
    });
    expect(result.current.system).toBe(true);

    rerender({ session: "s2" });
    expect(result.current).toEqual({ microphone: false, system: false });
  });

  it("sem sessão não assina", () => {
    renderHook(() => useTranscriptActivity(null));
    expect(subscribeMock.events).not.toContain("transcript_activity");
  });
});
