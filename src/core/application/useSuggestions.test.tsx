import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useSuggestions } from "./useSuggestions";
import type {
  ManualSuggestionContext,
  Suggestion,
  SuggestionPhase,
} from "../domain/assistant";

const suggestionGenerate = vi.fn();
const suggestionPin = vi.fn();
const suggestionLast = vi.fn();
const subscribeMock = vi.hoisted(() => ({
  emit: null as null | ((payload: unknown) => void),
}));

vi.mock("../../adapters/ipc/suggestions", () => ({
  suggestionGenerate: (...args: unknown[]) => suggestionGenerate(...args),
  suggestionPin: (...args: unknown[]) => suggestionPin(...args),
  suggestionLast: (...args: unknown[]) => suggestionLast(...args),
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
  generation_id: `g-${atMs}`,
  trigger: "auto",
  source: null,
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
  modeId: "meeting",
  extraContext: "",
  manualContext: null,
};

describe("useSuggestions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    suggestionLast.mockResolvedValue(null);
    suggestionGenerate.mockResolvedValue(suggestion("> fale do prazo", 1_000));
    suggestionPin.mockResolvedValue(suggestion("> fale do prazo", 1_000));
  });

  it("não gera ao abrir a janela", async () => {
    // Pedido do usuário: abrir não gera; o auto conta o intervalo e só gera
    // com fala nova (ou o botão “Sugerir agora”).
    renderHook(() => useSuggestions(options));
    await act(async () => {
      await Promise.resolve();
    });
    expect(suggestionGenerate).not.toHaveBeenCalled();
  });

  it("não gera ao trocar de agente (segue fala nova + contagem)", async () => {
    const { rerender } = renderHook(
      (props: { modeId: string }) => useSuggestions({ ...options, modeId: props.modeId }),
      { initialProps: { modeId: "meeting" } },
    );
    rerender({ modeId: "leetcode" });
    await act(async () => {
      await Promise.resolve();
    });
    expect(suggestionGenerate).not.toHaveBeenCalled();
  });

  it("força a sugestão no botão “Sugerir agora”", async () => {
    const { result } = renderHook(() => useSuggestions(options));

    await act(async () => {
      await result.current.suggestNow();
    });
    expect(suggestionGenerate).toHaveBeenCalledWith(
      expect.objectContaining({ force: true }),
    );
    expect(result.current.current?.text).toContain("prazo");
  });

  it("respeita skipped e mantém a sugestão atual", async () => {
    suggestionGenerate
      .mockResolvedValueOnce(suggestion("> primeira", 1_000))
      .mockResolvedValueOnce(suggestion("", 1_000, true));

    const { result } = renderHook(() => useSuggestions(options));
    await act(async () => {
      await result.current.suggestNow();
    });
    expect(result.current.current?.text).toContain("primeira");

    await act(async () => {
      await result.current.suggestNow();
    });

    expect(result.current.skipped).toBe(true);
    expect(result.current.current?.text).toContain("primeira");
  });

  it("fixa a sugestão atual", async () => {
    const { result } = renderHook(() => useSuggestions(options));
    await act(async () => {
      await result.current.suggestNow();
    });

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
    await act(async () => {
      await result.current.suggestNow();
    });
    expect(result.current.current?.text).toContain("primeira");

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

  it("mostra a rápida que chega depois da completa da mesma geração", async () => {
    // BR-088: com modelo rápido/streaming, a completa pode chegar primeiro;
    // a rápida ainda preenche o bloco próprio em vez de desaparecer.
    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!(suggestion("> resposta completa", 5_000));
    });
    await waitFor(() =>
      expect(result.current.current?.text).toContain("completa"),
    );
    expect(result.current.awaitingFull).toBe(false);

    act(() => {
      subscribeMock.emit!(suggestion("> rápida atrasada", 5_000, false, "quick"));
    });

    await waitFor(() =>
      expect(result.current.quick?.text).toContain("rápida atrasada"),
    );
    expect(result.current.current?.text).toContain("completa");
    expect(result.current.awaitingFull).toBe(false);
  });

  it("ignora a rápida de uma geração antiga depois de uma completa mais nova", async () => {
    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!(suggestion("> completa nova", 9_000));
    });
    await waitFor(() =>
      expect(result.current.current?.text).toContain("completa nova"),
    );

    act(() => {
      subscribeMock.emit!(suggestion("> rápida antiga", 1_000, false, "quick"));
    });

    expect(result.current.current?.text).toContain("completa nova");
    expect(result.current.quick).toBeNull();
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

  it("sinaliza a completa pendente desde a rápida (aviso garantido)", async () => {
    // O auto pode demorar entre a rápida e a completa; o aviso não pode
    // depender do streaming (que expira em 1,5 s).
    const { result } = renderHook(() => useSuggestions(options));
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!(suggestion("> rápida", 5_000, false, "quick"));
    });
    expect(result.current.awaitingFull).toBe(true);

    act(() => {
      subscribeMock.emit!(suggestion("> completa", 5_000));
    });
    expect(result.current.awaitingFull).toBe(false);
    expect(result.current.current?.text).toContain("completa");
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

  it("não duplica no histórico uma sugestão de conteúdo idêntico", async () => {
    suggestionGenerate
      .mockResolvedValueOnce(suggestion("> mesma resposta", 1_000))
      .mockResolvedValueOnce(suggestion("> mesma resposta", 2_000));

    const { result } = renderHook(() => useSuggestions(options));
    await act(async () => {
      await result.current.suggestNow();
    });

    await act(async () => {
      await result.current.suggestNow();
    });
    await waitFor(() => expect(result.current.current?.at_ms).toBe(2_000));

    // Fica só a mais recente: o histórico não mostra a mesma sugestão duas vezes.
    expect(result.current.history).toHaveLength(1);
    expect(result.current.history[0]?.at_ms).toBe(2_000);
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
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!(suggestion("> da sessão", 1_000));
    });
    await waitFor(() =>
      expect(result.current.current?.text).toContain("da sessão"),
    );

    act(() => {
      subscribeMock.emit!({
        ...suggestion("> de outra sessão", 9_000),
        session_id: "s2",
      });
    });

    expect(result.current.current?.text).toContain("da sessão");
  });

  it("hidrata o cartão com a última sugestão em cache, sem gerar", async () => {
    suggestionLast.mockResolvedValueOnce(suggestion("> do cache", 1_000));
    const { result } = renderHook(() => useSuggestions(options));

    await waitFor(() =>
      expect(result.current.current?.text).toContain("do cache"),
    );
    expect(suggestionGenerate).not.toHaveBeenCalled();
  });

  it("com contexto manual, a automática vai para o histórico e não rouba o cartão", async () => {
    const manualContext: ManualSuggestionContext = {
      session_id: "s1",
      speaker: "THEY",
      text: "Como tu faria um system design?",
      at_ms: 500,
    };
    type ManualProps = { manualContext: ManualSuggestionContext | null };
    const { result, rerender } = renderHook(
      (props: ManualProps) =>
        useSuggestions({ ...options, manualContext: props.manualContext }),
      { initialProps: { manualContext } as ManualProps },
    );
    await waitFor(() => expect(subscribeMock.emit).not.toBeNull());

    act(() => {
      subscribeMock.emit!(suggestion("> rascunho auto", 5_000, false, "quick"));
      subscribeMock.emit!(suggestion("> completa auto", 5_000));
    });

    await waitFor(() =>
      expect(result.current.history.some((item) => item.text.includes("completa auto"))).toBe(true),
    );
    expect(result.current.current).toBeNull();

    // Sem o contexto manual, a próxima automática pode ocupar o cartão.
    rerender({ manualContext: null });
    act(() => {
      subscribeMock.emit!({
        ...suggestion("> auto nova", 9_000),
        generation_id: "g-nova",
      });
    });
    await waitFor(() =>
      expect(result.current.current?.text).toContain("auto nova"),
    );
  });

  it("gera a sugestão manual com o contexto como parâmetro", async () => {
    const manualContext = {
      session_id: "s1",
      speaker: "THEY" as const,
      text: "Como tu faria um system design?",
      at_ms: 500,
    };
    const manualSuggestion: Suggestion = {
      ...suggestion("> manual gerada", 2_000),
      generation_id: "g-manual",
      trigger: "manual",
      source: { speaker: "THEY", at_ms: 500, preview: manualContext.text },
    };
    suggestionGenerate.mockResolvedValueOnce(manualSuggestion);

    const { result } = renderHook(() =>
      useSuggestions({ ...options, manualContext }),
    );

    await act(async () => {
      await result.current.generateManual();
    });

    expect(suggestionGenerate).toHaveBeenCalledWith(
      expect.objectContaining({ force: true, manualContext }),
    );
    expect(result.current.current?.trigger).toBe("manual");
    expect(result.current.current?.source?.preview).toBe(manualContext.text);
  });
});
