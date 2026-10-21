import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AskWindow } from "./AskWindow";

const journalAsk = vi.fn();
const screenCapture = vi.fn();
const capturePending = vi.fn();
const captureClear = vi.fn();
const askHide = vi.fn();
const askOpen = vi.fn();
const askStartDrag = vi.fn();
const panelOpenTab = vi.fn();
const subscribe = vi.fn();

vi.mock("../../adapters/ipc/journal", () => ({
  journalAsk: (...args: unknown[]) => journalAsk(...args),
}));

vi.mock("../../adapters/ipc/screen", () => ({
  screenCapture: (...args: unknown[]) => screenCapture(...args),
  capturePending: (...args: unknown[]) => capturePending(...args),
  captureClear: (...args: unknown[]) => captureClear(...args),
}));

vi.mock("../../adapters/ipc/ask", () => ({
  askHide: (...args: unknown[]) => askHide(...args),
  askOpen: (...args: unknown[]) => askOpen(...args),
  askStartDrag: (...args: unknown[]) => askStartDrag(...args),
}));

vi.mock("../../adapters/ipc/panels", () => ({
  panelOpenTab: (...args: unknown[]) => panelOpenTab(...args),
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: (...args: unknown[]) => subscribe(...args),
}));

const shot = { mime: "image/jpeg", data: "AAA", width: 1280, height: 720 };

describe("AskWindow (barra de pesquisa)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    journalAsk.mockResolvedValue({ entry_id: "e1", answer: "" });
    screenCapture.mockResolvedValue(shot);
    capturePending.mockResolvedValue(null);
    captureClear.mockResolvedValue(undefined);
    askHide.mockResolvedValue(undefined);
    askOpen.mockResolvedValue(undefined);
    askStartDrag.mockResolvedValue(undefined);
    panelOpenTab.mockResolvedValue(undefined);
    subscribe.mockResolvedValue(() => {});
  });

  it("o atalho devolve o cursor ao campo de pergunta", async () => {
    const handlers: Record<string, (payload: unknown) => void> = {};
    subscribe.mockImplementation(
      (event: string, handler: (payload: unknown) => void) => {
        handlers[event] = handler;
        return Promise.resolve(() => {});
      },
    );
    render(<AskWindow />);

    const input = screen.getByLabelText("Perguntar ao GhostWhisper");
    input.blur();
    expect(document.activeElement).not.toBe(input);

    act(() => {
      handlers["ask_focus"]?.({});
    });

    await waitFor(() => expect(document.activeElement).toBe(input));
  });

  it("a alça inicia o arraste com intenção explícita (BR-072/ISS-057)", () => {
    render(<AskWindow />);

    fireEvent.pointerDown(
      screen.getByLabelText("Mover a barra de pesquisa"),
    );

    expect(askStartDrag).toHaveBeenCalledTimes(1);
  });

  it("capturar apenas anexa — não envia a pergunta", async () => {
    render(<AskWindow />);

    fireEvent.click(screen.getByLabelText("Capturar Tela"));

    await waitFor(() => expect(screenCapture).toHaveBeenCalledTimes(1));
    expect(journalAsk).not.toHaveBeenCalled();
    // O botão passa a indicar o anexo.
    expect(await screen.findByText("Print anexado")).toBeTruthy();
  });

  it("envia com Enter, levando o print anexado, e fecha a barra", async () => {
    capturePending.mockResolvedValue(shot);
    render(<AskWindow />);

    const input = screen.getByLabelText("Perguntar ao GhostWhisper");
    fireEvent.change(input, { target: { value: "o que é isso?" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(journalAsk).toHaveBeenCalled());
    const request = journalAsk.mock.calls[0]?.[0];
    expect(request.question).toBe("o que é isso?");
    expect(request.source).toBe("screenshot");
    expect(request.images).toEqual([{ mime: "image/jpeg", data: "AAA" }]);
    expect(captureClear).toHaveBeenCalledTimes(1);
    expect(askHide).toHaveBeenCalledTimes(1);
    expect(panelOpenTab).toHaveBeenCalledWith("chat", "session");
  });

  it("sem anexo, envia só o texto", async () => {
    render(<AskWindow />);

    const input = screen.getByLabelText("Perguntar ao GhostWhisper");
    fireEvent.change(input, { target: { value: "resuma" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(journalAsk).toHaveBeenCalled());
    const request = journalAsk.mock.calls[0]?.[0];
    expect(request.images).toEqual([]);
    expect(request.source).toBe("search");
  });

  it("não envia pergunta vazia sem anexo", () => {
    render(<AskWindow />);
    fireEvent.keyDown(screen.getByLabelText("Perguntar ao GhostWhisper"), {
      key: "Enter",
    });
    expect(journalAsk).not.toHaveBeenCalled();
  });

  it("Esc fecha a barra sem perguntar", () => {
    render(<AskWindow />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(askHide).toHaveBeenCalled();
    expect(journalAsk).not.toHaveBeenCalled();
  });
});
