import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useLiveTranscription } from "./useLiveTranscription";
import type { TranscriptSegment } from "../domain/transcript";

type Handler = (payload: TranscriptSegment) => void;

const listeners = new Map<string, Handler>();

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: async (event: string, handler: Handler) => {
    listeners.set(event, handler);
    return () => listeners.delete(event);
  },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: async () => () => {},
}));

function segment(id: string, text: string, final: boolean): TranscriptSegment {
  return {
    id,
    session_id: "s1",
    speaker: "ME",
    text,
    start_ms: 0,
    end_ms: 500,
    final,
  };
}

describe("useLiveTranscription", () => {
  beforeEach(() => {
    listeners.clear();
  });

  it("substitui preview pelo final com o mesmo id", async () => {
    const { result } = renderHook(() => useLiveTranscription());

    // Captura o handler registrado no efeito.
    await act(async () => {});
    const handler = listeners.get("transcript_segment");
    expect(handler).toBeDefined();

    act(() => {
      handler?.(segment("s1-t0", "olá mun", false));
    });
    expect(result.current.segments).toHaveLength(1);
    expect(result.current.segments[0]?.final).toBe(false);

    // Mesmo id, agora final: substitui, não duplica.
    act(() => {
      handler?.(segment("s1-t0", "olá mundo", true));
    });
    expect(result.current.segments).toHaveLength(1);
    expect(result.current.segments[0]?.text).toBe("olá mundo");
    expect(result.current.segments[0]?.final).toBe(true);
  });

  it("acrescenta turnos distintos em ordem", async () => {
    const { result } = renderHook(() => useLiveTranscription());
    await act(async () => {});
    const handler = listeners.get("transcript_segment");

    act(() => {
      handler?.(segment("s1-t0", "primeiro", true));
    });
    act(() => {
      handler?.(segment("s1-t1", "segundo", true));
    });

    expect(result.current.segments.map((s) => s.text)).toEqual([
      "primeiro",
      "segundo",
    ]);
  });

  it("limpa a lista", async () => {
    const { result } = renderHook(() => useLiveTranscription());
    await act(async () => {});
    const handler = listeners.get("transcript_segment");
    act(() => {
      handler?.(segment("s1-t0", "oi", true));
    });
    expect(result.current.segments).toHaveLength(1);
    act(() => result.current.clear());
    expect(result.current.segments).toHaveLength(0);
  });
});