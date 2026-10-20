import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { JournalView } from "./JournalView";
import type { JournalEntry } from "../../core/domain/journal";

const journalList = vi.fn();
const journalImages = vi.fn();
const journalAsk = vi.fn();
const journalSearch = vi.fn();
const journalCancel = vi.fn();

/** Sessão ativa vista pela aba (a mesma que o core informa). */
const session = vi.hoisted(() => ({
  state: {
    listening: true,
    paused: false,
    session_id: "s1" as string | null,
    channels: [],
    started_at_ms: 1_700_000_000_000 as number | null,
  },
}));

vi.mock("../../core/application/useSessionState", () => ({
  useSessionState: () => ({
    state: session.state,
    busy: false,
    error: null,
    start: vi.fn(),
    pause: vi.fn(),
    resume: vi.fn(),
    stop: vi.fn(),
    continueSession: vi.fn(),
    setChannels: vi.fn(),
  }),
}));

vi.mock("../../adapters/ipc/journal", () => ({
  journalList: (...args: unknown[]) => journalList(...args),
  journalSearch: (...args: unknown[]) => journalSearch(...args),
  journalAsk: (...args: unknown[]) => journalAsk(...args),
  journalCancel: (...args: unknown[]) => journalCancel(...args),
  journalGet: vi.fn(),
  journalImages: (...args: unknown[]) => journalImages(...args),
  journalDelete: vi.fn(),
  journalImageDataUrl: (image: { mime: string }) =>
    `data:${image.mime};base64,QUJD`,
  journalImageBase64: () => "QUJD",
}));

const entry = (over: Partial<JournalEntry> = {}): JournalEntry => ({
  id: "e1",
  session_id: "s1",
  question: "o que eu disse?",
  answer: "isto",
  asked_at_ms: 4_000,
  answered_at_ms: 4_500,
  created_at_ms: 1_700_000_004_000,
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
    session.state.session_id = "s1";
    session.state.started_at_ms = 1_700_000_000_000;
    journalList.mockResolvedValue([entry()]);
    journalImages.mockResolvedValue([]);
    journalAsk.mockResolvedValue({ entry_id: "e2", answer: "resposta" });
    journalSearch.mockResolvedValue([]);
    journalCancel.mockResolvedValue(undefined);
  });

  it("mostra origem, pergunta e resposta da IA", async () => {
    render(<JournalView />);

    expect(await screen.findByText("o que eu disse?")).toBeTruthy();
    expect(screen.getByText("Pesquisa")).toBeTruthy();
    expect(screen.getByText("Q:")).toBeTruthy();
    expect(screen.getByText("isto")).toBeTruthy();
    expect(screen.getByText("GhostWhisper IA")).toBeTruthy();
  });

  it("põe a interação mais recente no topo", async () => {
    journalList.mockResolvedValue([
      entry({ id: "antiga", question: "primeira pergunta" }),
      entry({ id: "nova", question: "última pergunta" }),
    ]);
    const { container } = render(<JournalView />);

    await screen.findByText("última pergunta");
    const cards = [...container.querySelectorAll(".journal__entry")];
    expect(cards).toHaveLength(2);
    expect(cards[0]?.textContent).toContain("última pergunta");
    expect(cards[1]?.textContent).toContain("primeira pergunta");
    // O selo "Mais Recente" fica na primeira.
    expect(cards[0]?.textContent).toContain("Mais Recente");
  });

  it("print sem texto: legenda Screenshot e nenhuma pergunta", async () => {
    journalList.mockResolvedValue([
      entry({
        question: "",
        request_source: "screenshot",
        status: "pending",
        answer: null,
      }),
    ]);
    render(<JournalView />);

    expect(await screen.findByText("Screenshot")).toBeTruthy();
    expect(screen.queryByText("Q:")).toBeNull();
    expect(screen.queryByText("Contexto:")).toBeNull();
    expect(screen.getByText("Pensando…")).toBeTruthy();
  });

  it("print com texto: origem 'Screenshot + Texto' e 'Contexto:'", async () => {
    journalList.mockResolvedValue([
      entry({
        question: "explica esse trecho",
        request_source: "screenshot",
      }),
    ]);
    render(<JournalView />);

    expect(await screen.findByText("Screenshot + Texto")).toBeTruthy();
    expect(screen.getByText("Contexto:")).toBeTruthy();
  });

  it("carrega a miniatura do print do diário", async () => {
    journalList.mockResolvedValue([
      entry({ question: "", request_source: "screenshot" }),
    ]);
    journalImages.mockResolvedValue([
      {
        id: "i1",
        entry_id: "e1",
        mime: "image/jpeg",
        data: [65, 66, 67],
        width: 1876,
        height: 766,
      },
    ]);
    render(<JournalView />);

    const thumb = await screen.findByAltText("Print anexado à pergunta");
    expect(thumb.getAttribute("src")).toBe("data:image/jpeg;base64,QUJD");
    await waitFor(() => expect(journalImages).toHaveBeenCalledWith("e1"));
    expect(screen.getByText("1876×766")).toBeTruthy();
  });

  it("conta as interações no rodapé", async () => {
    journalList.mockResolvedValue([
      entry({ id: "a" }),
      entry({ id: "b" }),
      entry({ id: "c" }),
    ]);
    render(<JournalView />);

    expect(
      await screen.findByText("3 interações registradas nesta sessão"),
    ).toBeTruthy();
    expect(screen.getByText("Sync local · SQLite")).toBeTruthy();
  });

  it("usa o singular com uma interação", async () => {
    render(<JournalView />);
    expect(
      await screen.findByText("1 interação registrada nesta sessão"),
    ).toBeTruthy();
  });

  it("mostra o estado vazio sem interações", async () => {
    journalList.mockResolvedValue([]);
    render(<JournalView />);

    expect(
      await screen.findByText("Nenhuma interação nesta sessão ainda"),
    ).toBeTruthy();
  });
});
