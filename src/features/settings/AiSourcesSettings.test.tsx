import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AiSourcesSettings } from "./AiSourcesSettings";
import type { AiSource, AiSourceHealth } from "../../core/domain/settings";

const aiSourcesIpc = vi.hoisted(() => ({
  clearSourceKey: vi.fn(async (_sourceId: string) => undefined),
  hasSourceKey: vi.fn(async (_sourceId: string) => false),
  listSources: vi.fn(async () => [] as AiSource[]),
  setSourceKey: vi.fn(async (_sourceId: string, _key: string) => undefined),
  sourceKeyPreview: vi.fn(async (_sourceId: string) => ""),
  sourcesHealth: vi.fn(async () => [] as AiSourceHealth[]),
  validateSource: vi.fn(async (_sourceId: string) => "ok"),
}));

const settingsIpc = vi.hoisted(() => ({
  secretsPersistent: vi.fn(async () => true),
}));

/** Foco da janela: guarda o handler para disparar no teste. */
const focus = vi.hoisted(() => ({
  handler: null as null | ((focused: boolean) => void),
}));

const clientIpc = vi.hoisted(() => ({
  onWindowFocus: vi.fn(async (handler: (focused: boolean) => void) => {
    focus.handler = handler;
    return () => {
      focus.handler = null;
    };
  }),
}));

vi.mock("../../adapters/ipc/aiSources", () => aiSourcesIpc);
vi.mock("../../adapters/ipc/settings", () => settingsIpc);
vi.mock("../../adapters/ipc/client", async () => {
  const actual = await vi.importActual<typeof import("../../adapters/ipc/client")>(
    "../../adapters/ipc/client",
  );
  return { ...actual, ...clientIpc };
});

const openai: AiSource = {
  id: "a",
  kind: "openai",
  label: "OpenAI",
  model: "gpt-4o-mini",
  base_url: null,
  enabled: true,
};

const deepseek: AiSource = {
  id: "b",
  kind: "deepseek",
  label: "DeepSeek",
  model: "deepseek-chat",
  base_url: null,
  enabled: true,
};

function renderTab(
  sources: AiSource[],
  onChange = vi.fn((_next: AiSource[]) => undefined),
): { onChange: ReturnType<typeof vi.fn> } {
  render(
    <AiSourcesSettings
      sources={sources}
      providers={["openai", "gemini", "openrouter", "deepseek", "custom"]}
      onChange={onChange}
      onStatus={() => undefined}
    />,
  );
  return { onChange };
}

describe("AiSourcesSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    aiSourcesIpc.hasSourceKey.mockResolvedValue(false);
    aiSourcesIpc.sourceKeyPreview.mockResolvedValue("");
    aiSourcesIpc.sourcesHealth.mockResolvedValue([]);
    settingsIpc.secretsPersistent.mockResolvedValue(true);
  });

  it("mostra a contagem de fontes e o estado do keyring", async () => {
    renderTab([openai, deepseek]);
    expect(await screen.findByText(/2 fontes cadastradas/)).toBeDefined();
    expect(screen.getByText(/Keyring Seguro Ativo/)).toBeDefined();
  });

  it("a primeira fonte é a primária", async () => {
    renderTab([openai, deepseek]);
    const cards = await screen.findAllByRole("article");
    expect(cards.length).toBe(2);
    expect(cards[0]?.textContent).toContain("Primário");
    expect(cards[1]?.textContent).not.toContain("Primário");
  });

  it("promove a fonte escolhida para primária", async () => {
    const { onChange } = renderTab([openai, deepseek]);
    const buttons = await screen.findAllByRole("button", {
      name: "Tornar primário",
    });
    await act(async () => {
      fireEvent.click(buttons[0] as HTMLElement);
    });
    expect(onChange).toHaveBeenCalled();
    const next = onChange.mock.calls[0]?.[0] as AiSource[];
    expect(next[0]?.id).toBe("b");
    expect(next[1]?.id).toBe("a");
  });

  it("mostra o estado de saúde da fonte", async () => {
    aiSourcesIpc.sourcesHealth.mockResolvedValue([
      {
        source_id: "a",
        state: "ready",
        consecutive_failures: 0,
        latency_ms: 142,
        last_error: null,
        is_fallback: false,
        open_until_ms: null,
      },
    ]);
    renderTab([openai, deepseek]);
    expect(await screen.findByText("Conectada · Latência 142 ms")).toBeDefined();
  });

  it("marca a fonte em fallback", async () => {
    aiSourcesIpc.sourcesHealth.mockResolvedValue([
      {
        source_id: "b",
        state: "ready",
        consecutive_failures: 0,
        latency_ms: 90,
        last_error: null,
        is_fallback: true,
        open_until_ms: null,
      },
    ]);
    renderTab([openai, deepseek]);
    expect(await screen.findByText("Fallback ativo")).toBeDefined();
  });

  it("mostra a falha com o motivo", async () => {
    aiSourcesIpc.sourcesHealth.mockResolvedValue([
      {
        source_id: "a",
        state: "failed",
        consecutive_failures: 3,
        latency_ms: null,
        last_error: "provider respondeu 401: chave inválida",
        is_fallback: false,
        open_until_ms: 123,
      },
    ]);
    renderTab([openai]);
    expect(await screen.findByText("Falhou")).toBeDefined();
  });

  it("valida a chave de uma fonte", async () => {
    aiSourcesIpc.hasSourceKey.mockResolvedValue(true);
    aiSourcesIpc.sourceKeyPreview.mockResolvedValue("••••••••·1234");
    renderTab([openai]);
    const validate = await screen.findByRole("button", { name: "Validar chave" });
    await act(async () => {
      fireEvent.click(validate);
    });
    await waitFor(() =>
      expect(aiSourcesIpc.validateSource).toHaveBeenCalledWith(
        expect.objectContaining({ id: "a" }),
      ),
    );
  });

  it("grava a chave nova no keyring sem salvar as settings", async () => {
    aiSourcesIpc.hasSourceKey.mockResolvedValue(false);
    const { onChange } = renderTab([openai]);
    const add = await screen.findByRole("button", { name: "Adicionar chave" });
    await act(async () => {
      fireEvent.click(add);
    });
    const input = await screen.findByPlaceholderText("sk-...");
    fireEvent.change(input, { target: { value: "sk-nova" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Salvar chave/ }));
    });
    await waitFor(() =>
      expect(aiSourcesIpc.setSourceKey).toHaveBeenCalledWith("a", "sk-nova"),
    );
    // A lista de fontes **não** muda (a chave vive no keyring).
    expect(onChange).not.toHaveBeenCalled();
  });

  it("remove a chave de uma fonte", async () => {
    aiSourcesIpc.hasSourceKey.mockResolvedValue(true);
    renderTab([openai]);
    const remove = await screen.findByRole("button", { name: "Remover chave" });
    await act(async () => {
      fireEvent.click(remove);
    });
    await waitFor(() =>
      expect(aiSourcesIpc.clearSourceKey).toHaveBeenCalledWith("a"),
    );
  });

  it("adiciona uma fonte nova no rascunho", async () => {
    const { onChange } = renderTab([openai]);
    const provider = await screen.findByLabelText("Selecionar Provedor");
    await act(async () => {
      fireEvent.change(provider, { target: { value: "deepseek" } });
    });
    // O modelo é **digitado** pelo usuário (sem catálogo).
    const model = screen.getByPlaceholderText("nome-do-modelo");
    fireEvent.change(model, { target: { value: "deepseek-chat" } });

    const add = screen.getByRole("button", { name: "Adicionar" });
    await act(async () => {
      fireEvent.click(add);
    });
    expect(onChange).toHaveBeenCalled();
    const next = onChange.mock.calls[0]?.[0] as AiSource[];
    expect(next.length).toBe(2);
    expect(next[1]?.kind).toBe("deepseek");
    expect(next[1]?.model).toBe("deepseek-chat");
  });

  it("não adiciona a fonte sem modelo", async () => {
    const { onChange } = renderTab([openai]);
    const add = await screen.findByRole("button", { name: "Adicionar" });
    expect((add as HTMLButtonElement).disabled).toBe(true);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("exclui uma fonte do rascunho", async () => {
    const { onChange } = renderTab([openai, deepseek]);
    const remove = await screen.findAllByRole("button", {
      name: "Excluir fonte",
    });
    await act(async () => {
      fireEvent.click(remove[1] as HTMLElement);
    });
    const next = onChange.mock.calls[0]?.[0] as AiSource[];
    expect(next.length).toBe(1);
    expect(next[0]?.id).toBe("a");
  });

  it("reordena as fontes arrastando", async () => {
    const { onChange } = renderTab([openai, deepseek]);
    const cards = await screen.findAllByRole("article");

    await act(async () => {
      fireEvent.dragStart(cards[1] as HTMLElement);
      fireEvent.dragOver(cards[0] as HTMLElement);
      fireEvent.drop(cards[0] as HTMLElement);
    });

    expect(onChange).toHaveBeenCalled();
    const next = onChange.mock.calls[0]?.[0] as AiSource[];
    expect(next[0]?.id).toBe("b");
    expect(next[1]?.id).toBe("a");
  });

  it("publica o id no dataTransfer do arraste (regressão ISS-086)", async () => {
    // WebKitGTK só entrega `dragover`/`drop` quando o arraste carrega dados
    // (WebKit bug 265857): sem `setData` o card "arrasta" mas nunca reordena.
    const setData = vi.fn();
    const dataTransfer = {
      setData,
      effectAllowed: "none",
      dropEffect: "none",
      setDragImage: vi.fn(),
    } as unknown as DataTransfer;

    const { onChange } = renderTab([openai, deepseek]);
    const cards = await screen.findAllByRole("article");

    await act(async () => {
      fireEvent.dragStart(cards[1] as HTMLElement, { dataTransfer });
      fireEvent.dragOver(cards[0] as HTMLElement, { dataTransfer });
      fireEvent.drop(cards[0] as HTMLElement, { dataTransfer });
    });

    expect(setData).toHaveBeenCalledWith("text/plain", "b");
    expect(dataTransfer.effectAllowed).toBe("move");
    expect(dataTransfer.dropEffect).toBe("move");
    const next = onChange.mock.calls[0]?.[0] as AiSource[];
    expect(next.map((source) => source.id)).toEqual(["b", "a"]);
  });

  it("não reordena ao soltar sem arraste", async () => {
    const { onChange } = renderTab([openai, deepseek]);
    const cards = await screen.findAllByRole("article");

    await act(async () => {
      fireEvent.drop(cards[0] as HTMLElement);
    });

    expect(onChange).not.toHaveBeenCalled();
  });

  it("soltar o card sobre ele mesmo não muda a lista", async () => {
    const { onChange } = renderTab([openai, deepseek]);
    const cards = await screen.findAllByRole("article");

    await act(async () => {
      fireEvent.dragStart(cards[0] as HTMLElement);
      fireEvent.dragOver(cards[0] as HTMLElement);
      fireEvent.drop(cards[0] as HTMLElement);
    });

    expect(onChange).not.toHaveBeenCalled();
  });

  it("reordena pelo teclado na alça", async () => {
    const { onChange } = renderTab([openai, deepseek]);
    const grips = await screen.findAllByRole("button", {
      name: "Reordenar fonte: ↑ sobe, ↓ desce",
    });

    await act(async () => {
      fireEvent.keyDown(grips[1] as HTMLElement, { key: "ArrowUp" });
    });

    const next = onChange.mock.calls[0]?.[0] as AiSource[];
    expect(next.map((source) => source.id)).toEqual(["b", "a"]);
  });

  it("a alça não move a primeira fonte para cima", async () => {
    const { onChange } = renderTab([openai, deepseek]);
    const grips = await screen.findAllByRole("button", {
      name: "Reordenar fonte: ↑ sobe, ↓ desce",
    });

    await act(async () => {
      fireEvent.keyDown(grips[0] as HTMLElement, { key: "ArrowUp" });
    });

    expect(onChange).not.toHaveBeenCalled();
  });

  it("recarrega a saúde quando a janela ganha foco", async () => {
    // A janela de Configurações nasce oculta: o badge "Nunca usada" ficaria
    // congelado se a saúde não fosse buscada de novo ao abrir a janela.
    aiSourcesIpc.sourcesHealth.mockResolvedValue([]);
    renderTab([openai]);
    await screen.findAllByRole("article");

    const callsBefore = aiSourcesIpc.sourcesHealth.mock.calls.length;

    // Agora a fonte responde (o usuário rodou um teste).
    aiSourcesIpc.sourcesHealth.mockResolvedValue([
      {
        source_id: "a",
        state: "ready",
        consecutive_failures: 0,
        latency_ms: 142,
        last_error: null,
        is_fallback: false,
        open_until_ms: null,
      },
    ]);

    await act(async () => {
      focus.handler?.(true);
    });

    expect(aiSourcesIpc.sourcesHealth.mock.calls.length).toBeGreaterThan(
      callsBefore,
    );
    expect(await screen.findByText("Conectada · Latência 142 ms")).toBeDefined();
  });

  it("não deixa excluir a última fonte", async () => {
    renderTab([openai]);
    expect(screen.queryByRole("button", { name: "Excluir fonte" })).toBeNull();
  });
});
