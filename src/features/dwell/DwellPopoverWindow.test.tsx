import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DwellPopoverWindow } from "./DwellPopoverWindow";

const result = vi.hoisted(() => vi.fn(async () => undefined));
const handlers = vi.hoisted(
  () => new Map<string, (payload: unknown) => void>(),
);
const cursor = vi.hoisted(() => ({
  value: { inside: true, overConfirm: true },
}));

vi.mock("../../adapters/ipc/dwell", () => ({
  dwellResult: result,
  dwellContext: vi.fn(async () => 500),
  dwellShow: vi.fn(async () => undefined),
  dwellHide: vi.fn(async () => undefined),
  dwellTriggerHover: vi.fn(async () => undefined),
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: vi.fn(
    async (event: string, handler: (payload: unknown) => void) => {
      handlers.set(event, handler);
      return () => {};
    },
  ),
}));

vi.mock("../../adapters/ipc/client", () => ({
  isTauriRuntime: () => true,
}));

vi.mock("../../adapters/ipc/cursor", () => ({
  sampleDwellCursor: vi.fn(async () => cursor.value),
}));

async function renderWindow() {
  render(<DwellPopoverWindow />);
  // Deixa a busca inicial do contexto (promise) assentar.
  await act(async () => {});
}

describe("DwellPopoverWindow", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    result.mockClear();
    handlers.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("mostra a dica e confirma no clique", async () => {
    await renderWindow();
    const confirm = screen.getByRole("button", { name: /Sim/ });

    expect(screen.getByText("Sem clique")).toBeDefined();
    fireEvent.click(confirm);
    expect(result).toHaveBeenCalledWith(true);
  });

  it("confirma após a permanência no Sim", async () => {
    await renderWindow();
    const confirm = screen.getByRole("button", { name: /Sim/ });

    fireEvent.pointerOver(confirm);
    expect(confirm.getAttribute("data-arming")).toBe("true");
    act(() => vi.advanceTimersByTime(500));

    expect(result).toHaveBeenCalledTimes(1);
    expect(result).toHaveBeenCalledWith(true);
  });

  it("Escape cancela", async () => {
    await renderWindow();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(result).toHaveBeenCalledWith(false);
  });

  it("não fecha quando o cursor entra na confirmação", async () => {
    await renderWindow();
    const root = document.querySelector(".dwell-window") as HTMLElement;

    // Saiu do gatilho; o grace começa.
    act(() => handlers.get("dwell_trigger")?.({ entered: false }));
    act(() => vi.advanceTimersByTime(300));

    // Entrou na confirmação: o grace pausa.
    fireEvent.pointerOver(root);
    act(() => vi.advanceTimersByTime(500));

    expect(result).not.toHaveBeenCalledWith(false);
  });

  it("movimento dentro da confirmação também pausa o grace", async () => {
    await renderWindow();
    const root = document.querySelector(".dwell-window") as HTMLElement;

    act(() => handlers.get("dwell_trigger")?.({ entered: false }));
    act(() => vi.advanceTimersByTime(300));

    // Sem `pointerenter` (rede de segurança): o move pausa.
    fireEvent.pointerMove(root);
    act(() => vi.advanceTimersByTime(500));

    expect(result).not.toHaveBeenCalledWith(false);
  });

  it("fecha após o grace quando o cursor não entra na confirmação", async () => {
    await renderWindow();

    act(() => handlers.get("dwell_trigger")?.({ entered: false }));
    act(() => vi.advanceTimersByTime(500));

    expect(result).toHaveBeenCalledWith(false);
  });

  it("voltar ao gatilho cancela o grace", async () => {
    await renderWindow();

    act(() => handlers.get("dwell_trigger")?.({ entered: false }));
    act(() => vi.advanceTimersByTime(200));
    act(() => handlers.get("dwell_trigger")?.({ entered: true }));
    act(() => vi.advanceTimersByTime(500));

    expect(result).not.toHaveBeenCalledWith(false);
  });

  it("funciona na segunda abertura mesmo sem eventos de hover", async () => {
    await renderWindow();
    const context = handlers.get("dwell_context");
    const reset = handlers.get("dwell_reset");
    if (!context || !reset) {
      throw new Error("handlers de contexto/reset ausentes");
    }
    cursor.value = { inside: true, overConfirm: true };

    const open = async (token: number) => {
      await act(async () =>
        context({
          dwell_ms: 500,
          token,
          x: 0,
          y: 0,
          width: 260,
          height: 60,
          scale: 1,
        }),
      );
      // Deixa a primeira amostra do cursor assentar.
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
        await Promise.resolve();
      });
    };

    await open(1);
    act(() => vi.advanceTimersByTime(500));
    expect(result).toHaveBeenCalledTimes(1);
    expect(result).toHaveBeenCalledWith(true);

    result.mockClear();
    act(() => reset(null));

    // Segunda abertura: o hover do webview pode estar obsoleto; o poll resolve.
    await open(2);
    act(() => vi.advanceTimersByTime(500));
    expect(result).toHaveBeenCalledTimes(1);
    expect(result).toHaveBeenCalledWith(true);
  });
});
