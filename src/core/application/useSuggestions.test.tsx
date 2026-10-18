import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useSuggestions } from "./useSuggestions";
import type { Suggestion } from "../domain/assistant";

const suggestionGenerate = vi.fn();
const suggestionPin = vi.fn();

vi.mock("../../adapters/ipc/suggestions", () => ({
  suggestionGenerate: (...args: unknown[]) => suggestionGenerate(...args),
  suggestionPin: (...args: unknown[]) => suggestionPin(...args),
}));

const suggestion = (
  text: string,
  atMs: number,
  skipped = false,
): Suggestion => ({
  session_id: "s1",
  mode_id: "meeting",
  mode_name: "Reunião",
  text,
  at_ms: atMs,
  context_until_ms: atMs,
  message_id: skipped ? null : "m1",
  skipped,
});

const options = {
  sessionId: "s1",
  opened: true,
  modeId: "meeting",
  auto: false,
  intervalSecs: 20,
  active: true,
  extraContext: "",
};

describe("useSuggestions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    suggestionGenerate.mockResolvedValue(suggestion("> fale do prazo", 1_000));
    suggestionPin.mockResolvedValue(suggestion("> fale do prazo", 1_000));
  });

  it("gera uma sugestão ao abrir a janela", async () => {
    const { result } = renderHook(() => useSuggestions(options));

    await waitFor(() => {
      expect(result.current.current?.text).toContain("prazo");
    });
    expect(suggestionGenerate).toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: "s1", modeId: "meeting", sinceMs: null }),
    );
  });

  it("respeita skipped e mantém a sugestão atual", async () => {
    suggestionGenerate
      .mockResolvedValueOnce(suggestion("> primeira", 1_000))
      .mockResolvedValueOnce(suggestion("", 1_000, true));

    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(result.current.current).not.toBeNull());

    await act(async () => {
      await result.current.suggestNow();
    });

    expect(result.current.skipped).toBe(true);
    expect(result.current.current?.text).toContain("primeira");
  });

  it("fixa a sugestão atual", async () => {
    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(result.current.current).not.toBeNull());

    await act(async () => {
      await result.current.pin();
    });

    expect(suggestionPin).toHaveBeenCalledWith({
      sessionId: "s1",
      modeId: "meeting",
      text: "> fale do prazo",
      atMs: 1_000,
    });
  });

  it("acumula o histórico e permite voltar a uma anterior", async () => {
    suggestionGenerate
      .mockResolvedValueOnce(suggestion("> primeira", 1_000))
      .mockResolvedValueOnce(suggestion("> segunda", 2_000));

    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(result.current.current?.text).toContain("primeira"));

    await act(async () => {
      await result.current.suggestNow();
    });
    await waitFor(() => expect(result.current.current?.text).toContain("segunda"));

    expect(result.current.history).toHaveLength(2);
    act(() => {
      result.current.select(result.current.history[1]!);
    });
    expect(result.current.current?.text).toContain("primeira");
  });

  it("não gera sem sessão", async () => {
    renderHook(() => useSuggestions({ ...options, sessionId: null }));
    await Promise.resolve();
    expect(suggestionGenerate).not.toHaveBeenCalled();
  });
});
