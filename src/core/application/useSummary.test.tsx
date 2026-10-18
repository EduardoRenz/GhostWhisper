import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useSummary } from "./useSummary";
import type { MeetingSummary } from "../domain/assistant";

const summaryGet = vi.fn();
const summaryGenerate = vi.fn();

vi.mock("../../adapters/ipc/summary", () => ({
  summaryGet: (...args: unknown[]) => summaryGet(...args),
  summaryGenerate: (...args: unknown[]) => summaryGenerate(...args),
}));

const summary = (markdown: string, updatedAt: number): MeetingSummary => ({
  session_id: "s1",
  subject: "T",
  markdown,
  summarized_from_ms: 0,
  summarized_until_ms: 5_000,
  updated_at_ms: updatedAt,
});

describe("useSummary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    summaryGet.mockResolvedValue(null);
    summaryGenerate.mockResolvedValue({
      summary: summary("## A", 10),
      updated: true,
      has_more: false,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("carrega o resumo gravado ao entrar na sessão", async () => {
    summaryGet.mockResolvedValue(summary("## Gravado", 1));

    const { result } = renderHook(() =>
      useSummary({
        sessionId: "s1",
        opened: false,
        auto: false,
        intervalSecs: 30,
        active: false,
      }),
    );

    await waitFor(() => {
      expect(result.current.summary?.markdown).toBe("## Gravado");
    });
    expect(summaryGet).toHaveBeenCalledWith("s1");
  });

  it("gera ao abrir a janela e regenera no manual", async () => {
    const { result } = renderHook(() =>
      useSummary({
        sessionId: "s1",
        opened: true,
        auto: false,
        intervalSecs: 30,
        active: true,
      }),
    );

    await waitFor(() => {
      expect(summaryGenerate).toHaveBeenCalledWith("s1", "auto");
    });

    await act(async () => {
      await result.current.regenerate();
    });
    expect(summaryGenerate).toHaveBeenCalledWith("s1", "manual");
  });

  it("atualiza no intervalo quando o automático está ligado", async () => {
    vi.useFakeTimers();
    summaryGenerate.mockResolvedValue({
      summary: summary("## A", 10),
      updated: false,
      has_more: false,
    });

    renderHook(() =>
      useSummary({
        sessionId: "s1",
        opened: true,
        auto: true,
        intervalSecs: 10,
        active: true,
      }),
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });

    expect(summaryGenerate.mock.calls.length).toBeGreaterThanOrEqual(3);
  });

  it("não chama o LLM sem sessão", async () => {
    renderHook(() =>
      useSummary({
        sessionId: null,
        opened: true,
        auto: true,
        intervalSecs: 10,
        active: true,
      }),
    );

    await Promise.resolve();
    expect(summaryGenerate).not.toHaveBeenCalled();
  });
});
