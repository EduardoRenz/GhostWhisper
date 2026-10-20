import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useJournal } from "./useJournal";
import type { JournalEntry, JournalUpdate } from "../domain/journal";

const journalList = vi.fn();
const journalSearch = vi.fn();
const journalAsk = vi.fn();
const journalCancel = vi.fn();
const subscribe = vi.fn();

vi.mock("../../adapters/ipc/journal", () => ({
  journalList: (...args: unknown[]) => journalList(...args),
  journalSearch: (...args: unknown[]) => journalSearch(...args),
  journalAsk: (...args: unknown[]) => journalAsk(...args),
  journalCancel: (...args: unknown[]) => journalCancel(...args),
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: (...args: unknown[]) => subscribe(...args),
}));

const entry = (id: string, question: string, at: number): JournalEntry => ({
  id,
  session_id: "s1",
  question,
  answer: "resposta",
  asked_at_ms: at,
  answered_at_ms: at + 500,
  created_at_ms: 1_700_000_000_000 + at,
  status: "answered",
  request_source: "search",
  provider: null,
  model: null,
  error: null,
});

describe("useJournal", () => {
  let emit: ((update: JournalUpdate) => void) | null = null;

  beforeEach(() => {
    vi.clearAllMocks();
    emit = null;
    journalList.mockResolvedValue([]);
    journalSearch.mockResolvedValue([]);
    journalAsk.mockResolvedValue({ entry_id: "e1", answer: "ok" });
    journalCancel.mockResolvedValue(undefined);
    subscribe.mockImplementation(
      (event: string, handler: (payload: JournalUpdate) => void) => {
        if (event === "journal") {
          emit = handler;
        }
        return Promise.resolve(() => {});
      },
    );
  });

  it("mostra a entrada mais recente primeiro", async () => {
    journalList.mockResolvedValue([
      entry("e1", "primeira", 0),
      entry("e2", "segunda", 1_000),
    ]);

    const { result } = renderHook(() => useJournal("s1"));

    await waitFor(() => expect(result.current.entries).toHaveLength(2));
    expect(result.current.entries.map((item) => item.id)).toEqual(["e2", "e1"]);
    expect(result.current.count).toBe(2);
  });

  it("zera o diário quando a sessão encerra", async () => {
    journalList.mockResolvedValue([entry("e1", "da sessão antiga", 0)]);

    const { result, rerender } = renderHook(
      ({ sessionId }) => useJournal(sessionId),
      { initialProps: { sessionId: "s1" as string | null } },
    );
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    // Encerrou a sessão: o core não tem sessão ativa → diário vazio.
    journalList.mockResolvedValue([]);
    rerender({ sessionId: null });

    await waitFor(() => expect(result.current.entries).toHaveLength(0));
    expect(result.current.count).toBe(0);
  });

  it("troca para o diário da sessão nova", async () => {
    journalList.mockResolvedValue([entry("e1", "antiga", 0)]);

    const { result, rerender } = renderHook(
      ({ sessionId }) => useJournal(sessionId),
      { initialProps: { sessionId: "s1" as string | null } },
    );
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    journalList.mockResolvedValue([entry("e2", "nova", 0)]);
    rerender({ sessionId: "s2" });

    await waitFor(() =>
      expect(result.current.entries.map((item) => item.id)).toEqual(["e2"]),
    );
  });

  it("não apaga a resposta quando o reload do fim volta vazio", async () => {
    // O reload pós-resposta é só para trazer origem/modelo: se a sessão não
    // for resolvida, a resposta que está na tela **fica** (era o bug do
    // "pensa e depois some").
    journalList.mockResolvedValue([entry("e1", "pergunta", 0)]);

    const { result } = renderHook(() => useJournal("s1"));
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    journalList.mockResolvedValue([]);
    act(() => {
      emit?.({
        entry_id: "e1",
        phase: "answered",
        text: "resposta final",
        error: null,
      });
    });

    await waitFor(() =>
      expect(result.current.entries[0]?.answer).toBe("resposta final"),
    );
    expect(result.current.entries).toHaveLength(1);
  });

  it("mescla o registro do banco (origem e modelo) sem perder a ordem", async () => {
    journalList.mockResolvedValue([entry("e1", "pergunta", 0)]);

    const { result } = renderHook(() => useJournal("s1"));
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    journalList.mockResolvedValue([
      {
        ...entry("e1", "pergunta", 0),
        answer: "resposta final",
        request_source: "screenshot",
        model: "gpt-4o",
      },
    ]);
    act(() => {
      emit?.({
        entry_id: "e1",
        phase: "answered",
        text: "resposta final",
        error: null,
      });
    });

    await waitFor(() =>
      expect(result.current.entries[0]?.request_source).toBe("screenshot"),
    );
    expect(result.current.entries[0]?.answer).toBe("resposta final");
    expect(result.current.entries[0]?.model).toBe("gpt-4o");
  });

  it("acompanha o streaming da resposta", async () => {
    journalList.mockResolvedValue([
      entry("e1", "pergunta", 0),
    ]);

    const { result } = renderHook(() => useJournal("s1"));
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    act(() => {
      emit?.({
        entry_id: "e1",
        phase: "streaming",
        text: "resposta pela metade",
        error: null,
      });
    });

    await waitFor(() =>
      expect(result.current.entries[0]?.answer).toBe("resposta pela metade"),
    );
  });
});
