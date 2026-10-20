import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { JournalView } from "./JournalView";
import type { JournalEntry } from "../../core/domain/journal";

const journalList = vi.fn();
const journalSearch = vi.fn();
const journalAsk = vi.fn();
const journalCancel = vi.fn();
const screenCapture = vi.fn();
const capturePending = vi.fn();
const captureClear = vi.fn();
const saveSettings = vi.fn();
const getSettings = vi.fn();

vi.mock("../../adapters/ipc/journal", () => ({
  journalList: (...args: unknown[]) => journalList(...args),
  journalSearch: (...args: unknown[]) => journalSearch(...args),
  journalAsk: (...args: unknown[]) => journalAsk(...args),
  journalCancel: (...args: unknown[]) => journalCancel(...args),
  journalGet: vi.fn(),
  journalImages: vi.fn(),
  journalDelete: vi.fn(),
}));

vi.mock("../../adapters/ipc/screen", async () => {
  const actual = await vi.importActual<
    typeof import("../../adapters/ipc/screen")
  >("../../adapters/ipc/screen");
  return {
    ...actual,
    screenCapture: (...args: unknown[]) => screenCapture(...args),
    capturePending: (...args: unknown[]) => capturePending(...args),
    captureClear: (...args: unknown[]) => captureClear(...args),
  };
});

vi.mock("../../adapters/ipc/settings", () => ({
  getSettings: (...args: unknown[]) => getSettings(...args),
  saveSettings: (...args: unknown[]) => saveSettings(...args),
}));

const entry = (over: Partial<JournalEntry> = {}): JournalEntry => ({
  id: "e1",
  session_id: "s1",
  question: "o que eu disse?",
  answer: "isto",
  asked_at_ms: 4_000,
  answered_at_ms: 4_500,
  status: "answered",
  request_source: "search",
  provider: "openai",
  model: "gpt-4o",
  error: null,
  ...over,
});

describe("JournalView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    journalList.mockResolvedValue([entry()]);
    journalSearch.mockResolvedValue([]);
    journalAsk.mockResolvedValue({ entry_id: "e2", answer: "resposta" });
    journalCancel.mockResolvedValue(undefined);
    screenCapture.mockResolvedValue(undefined);
    capturePending.mockResolvedValue(null);
    captureClear.mockResolvedValue(undefined);
    saveSettings.mockResolvedValue(undefined);
    getSettings.mockResolvedValue(null);
  });

  it("mostra o diário da sessão", async () => {
    render(<JournalView />);
    expect(await screen.findByText("o que eu disse?")).toBeTruthy();
    expect(screen.getByText("isto")).toBeTruthy();
  });

  it("mostra o estado vazio quando não há perguntas", async () => {
    journalList.mockResolvedValue([]);
    render(<JournalView />);
    expect(
      await screen.findByText(/Nenhuma pergunta ainda/),
    ).toBeTruthy();
  });

  it("envia a pergunta digitada", async () => {
    render(<JournalView />);
    const input = await screen.findByLabelText("Pergunta para a IA");
    fireEvent.change(input, { target: { value: "qual o prazo?" } });
    fireEvent.click(screen.getByLabelText("Perguntar"));

    await waitFor(() => expect(journalAsk).toHaveBeenCalled());
    expect(journalAsk.mock.calls[0]?.[0]?.question).toBe("qual o prazo?");
  });

  it("envia a pergunta com Enter", async () => {
    render(<JournalView />);
    const input = await screen.findByLabelText("Pergunta para a IA");
    fireEvent.change(input, { target: { value: "com enter" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(journalAsk).toHaveBeenCalled());
  });

  it("não envia com Shift+Enter", async () => {
    render(<JournalView />);
    const input = await screen.findByLabelText("Pergunta para a IA");
    fireEvent.change(input, { target: { value: "quebra linha" } });
    fireEvent.keyDown(input, { key: "Enter", shiftKey: true });

    expect(journalAsk).not.toHaveBeenCalled();
  });

  it("clicar num chip de pergunta pronta envia a pergunta", async () => {
    getSettings.mockResolvedValue({
      saved_questions: [{ id: "q1", text: "Listar as ações" }],
    });
    render(<JournalView />);

    const chip = await screen.findByTitle("Perguntar: Listar as ações");
    fireEvent.click(chip);

    await waitFor(() => expect(journalAsk).toHaveBeenCalled());
    expect(journalAsk.mock.calls[0]?.[0]?.question).toBe("Listar as ações");
    expect(journalAsk.mock.calls[0]?.[0]?.source).toBe("preset");
  });

  it("o print pendente do core vira a miniatura ao abrir a aba", async () => {
    capturePending.mockResolvedValue({
      mime: "image/jpeg",
      data: "AAA",
      width: 1280,
      height: 720,
    });
    render(<JournalView />);
    expect(await screen.findByAltText("Print anexado")).toBeTruthy();
  });

  it("capturar apenas anexa (o core guarda o pendente)", async () => {
    screenCapture.mockResolvedValue({
      mime: "image/jpeg",
      data: "AAA",
      width: 1280,
      height: 720,
    });
    render(<JournalView />);

    fireEvent.click(await screen.findByLabelText("Capturar a tela"));

    await waitFor(() => expect(screenCapture).toHaveBeenCalledTimes(1));
    // Não envia a pergunta: o anexo só ficou pendente.
    expect(journalAsk).not.toHaveBeenCalled();
  });

  it("avisa quando a captura de tela falha", async () => {
    screenCapture.mockRejectedValue(new Error("captura indisponível"));
    render(<JournalView />);

    fireEvent.click(await screen.findByLabelText("Capturar a tela"));

    expect(
      await screen.findByText(/captura indisponível/i),
    ).toBeTruthy();
  });

  it("cola a imagem da área de transferência", async () => {
    render(<JournalView />);
    const input = await screen.findByLabelText("Pergunta para a IA");

    const file = new File(["bytes"], "print.png", { type: "image/png" });
    fireEvent.paste(input, {
      clipboardData: {
        items: [{ type: "image/png", getAsFile: () => file }],
      },
    });

    expect(await screen.findByAltText("Print anexado")).toBeTruthy();
  });

  it("busca no diário com debounce", async () => {
    render(<JournalView />);
    const search = await screen.findByLabelText("Buscar no diário");
    fireEvent.change(search, { target: { value: "prazo" } });

    await waitFor(() => expect(journalSearch).toHaveBeenCalled(), {
      timeout: 1_000,
    });
    expect(journalSearch.mock.calls[0]?.[1]).toBe("prazo");
  });
});
