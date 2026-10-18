import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SUMMARY_HISTORY_LIMIT, useSummary } from "./useSummary";
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

  it("acumula o histórico de versões e congela na versão escolhida", async () => {
    summaryGenerate
      .mockResolvedValueOnce({
        summary: summary("## V1", 1),
        updated: true,
        has_more: false,
      })
      .mockResolvedValueOnce({
        summary: summary("## V2", 2),
        updated: true,
        has_more: false,
      });

    const { result } = renderHook(() =>
      useSummary({
        sessionId: "s1",
        opened: true,
        auto: false,
        intervalSecs: 30,
        active: true,
      }),
    );

    await waitFor(() => expect(result.current.summary?.markdown).toBe("## V1"));
    await act(async () => {
      await result.current.regenerate();
    });
    await waitFor(() => expect(result.current.summary?.markdown).toBe("## V2"));
    expect(result.current.history.map((item) => item.markdown)).toEqual([
      "## V2",
      "## V1",
    ]);

    // Volta para a V1 e uma atualização nova chega: a leitura fica congelada.
    act(() => result.current.select(result.current.history[1]!));
    expect(result.current.viewing?.markdown).toBe("## V1");

    summaryGenerate.mockResolvedValueOnce({
      summary: summary("## V3", 3),
      updated: true,
      has_more: false,
    });
    await act(async () => {
      await result.current.regenerate();
    });
    expect(result.current.summary?.markdown).toBe("## V3");
    expect(result.current.viewing?.markdown).toBe("## V1");

    act(() => result.current.showLatest());
    expect(result.current.viewing).toBeNull();
  });

  it("não empilha versão quando o tick não atualizou (skipped)", async () => {
    summaryGenerate.mockResolvedValue({
      summary: summary("## V1", 1),
      updated: false,
      has_more: false,
    });

    const { result } = renderHook(() =>
      useSummary({
        sessionId: "s1",
        opened: true,
        auto: false,
        intervalSecs: 30,
        active: true,
      }),
    );

    await waitFor(() => expect(summaryGenerate).toHaveBeenCalled());
    await act(async () => {
      await result.current.refresh();
    });
    expect(result.current.history).toHaveLength(0);
  });

  it("limita o histórico ao teto", async () => {
    let counter = 0;
    summaryGenerate.mockImplementation(async () => {
      counter += 1;
      return {
        summary: summary(`## V${counter}`, counter),
        updated: true,
        has_more: false,
      };
    });

    const { result } = renderHook(() =>
      useSummary({
        sessionId: "s1",
        opened: true,
        auto: false,
        intervalSecs: 30,
        active: true,
      }),
    );

    await waitFor(() => expect(result.current.history).toHaveLength(1));
    for (let i = 0; i < SUMMARY_HISTORY_LIMIT + 4; i += 1) {
      await act(async () => {
        await result.current.refresh();
      });
    }
    expect(result.current.history).toHaveLength(SUMMARY_HISTORY_LIMIT);
  });

  it("limpa histórico e versão em leitura ao trocar de sessão", async () => {
    summaryGenerate.mockResolvedValue({
      summary: summary("## V1", 1),
      updated: true,
      has_more: false,
    });

    const { result, rerender } = renderHook(
      ({ sessionId }: { sessionId: string }) =>
        useSummary({
          sessionId,
          opened: true,
          auto: false,
          intervalSecs: 30,
          active: true,
        }),
      { initialProps: { sessionId: "s1" } },
    );

    await waitFor(() => expect(result.current.history).toHaveLength(1));
    act(() => result.current.select(result.current.history[0]!));
    expect(result.current.viewing).not.toBeNull();

    rerender({ sessionId: "s2" });
    await waitFor(() => expect(result.current.history).toHaveLength(0));
    expect(result.current.viewing).toBeNull();
  });
});
