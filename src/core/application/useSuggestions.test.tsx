import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useSuggestions } from "./useSuggestions";
import type { Suggestion, SuggestionPhase } from "../domain/assistant";

const suggestionGenerate = vi.fn();
const suggestionPin = vi.fn();
const subscribeMock = vi.hoisted(() => ({
  emit: null as null | ((payload: unknown) => void),
}));

vi.mock("../../adapters/ipc/suggestions", () => ({
  suggestionGenerate: (...args: unknown[]) => suggestionGenerate(...args),
  suggestionPin: (...args: unknown[]) => suggestionPin(...args),
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: (_event: string, handler: (payload: unknown) => void) => {
    subscribeMock.emit = handler;
    return Promise.resolve(() => {});
  },
}));

const suggestion = (
  text: string,
  atMs: number,
  skipped = false,
  phase: SuggestionPhase = "full",
): Suggestion => ({
  session_id: "s1",
  mode_id: "meeting",
  mode_name: "Reunião",
  mode_icon: "users",
  mode_color: "cyan",
  phase,
  title: "",
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

  it("não força a sugestão ao abrir (o core decide se há fala nova)", async () => {
    // BR-039: abrir a janela não pode gerar uma sugestão nova sem fala nova.
    renderHook(() => useSuggestions(options));
    await waitFor(() => expect(suggestionGenerate).toHaveBeenCalled());
    expect(suggestionGenerate).toHaveBeenCalledWith(
      expect.objectContaining({ force: false }),
    );
  });

  it("não gera ao trocar de agente (segue fala nova + contagem)", async () => {
    const { rerender } = renderHook(
      (props: { modeId: string }) => useSuggestions({ ...options, modeId: props.modeId }),
      { initialProps: { modeId: "meeting" } },
    );
    await waitFor(() => expect(suggestionGenerate).toHaveBeenCalled());
    suggestionGenerate.mockClear();

    rerender({ modeId: "leetcode" });
    // Nenhuma chamada: a próxima sugestão vem da regra normal (ou do botão).
    await act(async () => {
      await Promise.resolve();
    });
    expect(suggestionGenerate).not.toHaveBeenCalled();
  });

  it("força a sugestão no botão “Sugerir agora”", async () => {
    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(result.current.current).not.toBeNull());
    suggestionGenerate.mockClear();

    await act(async () => {
      await result.current.suggestNow();
    });
    expect(suggestionGenerate).toHaveBeenCalledWith(
      expect.objectContaining({ force: true }),
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

  it("acumula o histórico sem substituir a sugestão do topo", async () => {
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
    // O topo continua sendo a sugestão mais recente (BR-038/CA-04.2).
    expect(result.current.current?.text).toContain("segunda");
  });

  it("mantém a resposta rápida quando a completa chega (BR-038)", async () => {
    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!(suggestion("> resposta curta", 5_000, false, "quick"));
    });
    await waitFor(() =>
      expect(result.current.quick?.text).toContain("resposta curta"),
    );
    // A rápida ainda não é a sugestão completa (o cartão mostra só ela).
    expect(result.current.current).toBeNull();

    act(() => {
      subscribeMock.emit!(suggestion("> resposta completa", 5_000));
    });
    await waitFor(() =>
      expect(result.current.current?.text).toContain("resposta completa"),
    );
    // A rápida continua visível ao lado da completa (mesma geração).
    expect(result.current.quick?.text).toContain("resposta curta");
  });

  it("tira a sugestão anterior do topo quando a nova começa (BR-038)", async () => {
    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    // Primeira geração, completa.
    act(() => {
      subscribeMock.emit!(suggestion("> resposta completa antiga", 1_000));
    });
    await waitFor(() => expect(result.current.current?.text).toContain("antiga"));

    // A nova geração começa pela resposta rápida: o topo zera e a anterior
    // continua no histórico (o accordion mostra ela).
    act(() => {
      subscribeMock.emit!(suggestion("> resposta curta nova", 2_000, false, "quick"));
    });
    expect(result.current.current).toBeNull();
    expect(result.current.quick?.text).toContain("curta nova");
    expect(
      result.current.history.some((item) => item.text.includes("antiga")),
    ).toBe(true);

    // E a completa da nova geração aparece no topo.
    act(() => {
      subscribeMock.emit!(suggestion("> resposta completa nova", 2_000));
    });
    expect(result.current.current?.text).toContain("completa nova");
    expect(result.current.quick?.text).toContain("curta nova");
  });

  it("descarta a resposta rápida de outra geração", async () => {
    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!(suggestion("> curta antiga", 1_000, false, "quick"));
    });
    await waitFor(() => expect(result.current.quick).not.toBeNull());

    act(() => {
      subscribeMock.emit!(suggestion("> completa nova", 9_000));
    });
    await waitFor(() =>
      expect(result.current.current?.text).toContain("completa nova"),
    );
    expect(result.current.quick).toBeNull();
  });

  it("não gera sem sessão", async () => {
    renderHook(() => useSuggestions({ ...options, sessionId: null }));
    await Promise.resolve();
    expect(suggestionGenerate).not.toHaveBeenCalled();
  });

  it("mostra a sugestão automática que chega por evento (streaming)", async () => {
    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!(suggestion("> rascunho", 5_000));
      subscribeMock.emit!(suggestion("> rascunho completo", 5_000));
    });

    await waitFor(() =>
      expect(result.current.current?.text).toContain("rascunho completo"),
    );
    // O rascunho e o texto completo compartilham `at_ms`: uma entrada só.
    const entries = result.current.history.filter((item) => item.at_ms === 5_000);
    expect(entries).toHaveLength(1);
    expect(entries[0]?.text).toContain("rascunho completo");
  });

  it("ignora evento de outra sessão", async () => {
    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(result.current.current).not.toBeNull());

    act(() => {
      subscribeMock.emit!({
        ...suggestion("> de outra sessão", 9_000),
        session_id: "s2",
      });
    });

    expect(result.current.current?.text).toContain("prazo");
  });
});
