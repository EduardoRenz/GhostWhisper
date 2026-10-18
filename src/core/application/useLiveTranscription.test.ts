import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useLiveTranscription } from "./useLiveTranscription";
import type { TranscriptSegment } from "../domain/transcript";

type Handler = (payload: TranscriptSegment) => void;

const listeners = new Map<string, Handler>();
/** Histórico devolvido pelo banco, por sessão. */
const persisted = new Map<string, TranscriptSegment[]>();

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: async (event: string, handler: Handler) => {
    listeners.set(event, handler);
    return () => listeners.delete(event);
  },
}));

vi.mock("../../adapters/ipc/database", () => ({
  sessionTranscriptions: async (sessionId: string) => persisted.get(sessionId) ?? [],
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: async () => () => {},
}));

function segment(
  id: string,
  text: string,
  final: boolean,
  sessionId = "s1",
  startMs = 0,
): TranscriptSegment {
  return {
    id,
    session_id: sessionId,
    speaker: "ME",
    text,
    start_ms: startMs,
    end_ms: startMs + 500,
    final,
  };
}

describe("useLiveTranscription", () => {
  beforeEach(() => {
    listeners.clear();
    persisted.clear();
  });

  it("substitui preview pelo final com o mesmo id", async () => {
    const { result } = renderHook(() => useLiveTranscription("s1"));

    await act(async () => {});
    const handler = listeners.get("transcript_segment");
    expect(handler).toBeDefined();

    act(() => {
      handler?.(segment("s1-0-t0", "olá mun", false));
    });
    expect(result.current.segments).toHaveLength(1);
    expect(result.current.segments[0]?.final).toBe(false);

    // Mesmo id, agora final: substitui, não duplica.
    act(() => {
      handler?.(segment("s1-0-t0", "olá mundo", true));
    });
    expect(result.current.segments).toHaveLength(1);
    expect(result.current.segments[0]?.text).toBe("olá mundo");
    expect(result.current.segments[0]?.final).toBe(true);
  });

  it("acrescenta turnos distintos em ordem", async () => {
    const { result } = renderHook(() => useLiveTranscription("s1"));
    await act(async () => {});
    const handler = listeners.get("transcript_segment");

    act(() => {
      handler?.(segment("s1-0-t0", "primeiro", true, "s1", 0));
    });
    act(() => {
      handler?.(segment("s1-0-t1", "segundo", true, "s1", 1_000));
    });

    expect(result.current.segments.map((s) => s.text)).toEqual([
      "primeiro",
      "segundo",
    ]);
  });

  it("carrega o histórico persistido ao (re)abrir a sessão (BR-019)", async () => {
    // Sessão continuada: o banco já tem as mensagens da captura anterior.
    persisted.set("s1", [
      segment("s1-0-t0", "fala anterior", true, "s1", 0),
      segment("s1-0-t1", "outra fala", true, "s1", 1_000),
    ]);

    const { result } = renderHook(() => useLiveTranscription("s1"));

    await waitFor(() => {
      expect(result.current.segments).toHaveLength(2);
    });
    expect(result.current.segments.map((s) => s.text)).toEqual([
      "fala anterior",
      "outra fala",
    ]);

    // O que chega ao vivo entra depois do histórico, sem duplicar o mesmo id.
    const handler = listeners.get("transcript_segment");
    act(() => {
      handler?.(segment("s1-0-t1", "outra fala", true, "s1", 1_000));
      handler?.(segment("s60-0-t0", "fala nova", true, "s1", 60_000));
    });
    expect(result.current.segments.map((s) => s.text)).toEqual([
      "fala anterior",
      "outra fala",
      "fala nova",
    ]);
  });

  it("não mistura mensagens de sessões diferentes", async () => {
    persisted.set("s2", [segment("s2-0-t0", "sessão nova", true, "s2", 0)]);
    const { result, rerender } = renderHook(
      ({ id }: { id: string | null }) => useLiveTranscription(id),
      { initialProps: { id: "s1" } },
    );
    await act(async () => {});
    const handler = listeners.get("transcript_segment");
    act(() => {
      handler?.(segment("s1-0-t0", "sessão antiga", true, "s1", 0));
    });
    expect(result.current.segments.map((s) => s.text)).toEqual(["sessão antiga"]);

    rerender({ id: "s2" });

    await waitFor(() => {
      expect(result.current.segments.map((s) => s.text)).toEqual(["sessão nova"]);
    });
  });

  it("esvazia a tela quando não há sessão", async () => {
    const { result, rerender } = renderHook(
      ({ id }: { id: string | null }) => useLiveTranscription(id),
      { initialProps: { id: "s1" as string | null } },
    );
    await act(async () => {});
    const handler = listeners.get("transcript_segment");
    act(() => {
      handler?.(segment("s1-0-t0", "oi", true));
    });
    expect(result.current.segments).toHaveLength(1);

    rerender({ id: null });
    await waitFor(() => {
      expect(result.current.segments).toHaveLength(0);
    });
  });
});
