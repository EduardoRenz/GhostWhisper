import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ChatView } from "./ChatView";
import type { ChatBubble } from "../../core/domain/chat";
import type { TranscriptSegment } from "../../core/domain/transcript";

const copyText = vi.fn();

vi.mock("../../core/application/clipboard", () => ({
  copyText: (...args: unknown[]) => copyText(...args),
}));

const speech = (
  id: string,
  speaker: "ME" | "THEY",
  text: string,
  atMs: number,
): ChatBubble => ({ kind: "speech", id, speaker, text, at_ms: atMs });

const suggestion = (id: string, text: string, atMs: number): ChatBubble => ({
  kind: "suggestion",
  id,
  text,
  at_ms: atMs,
});

const pendingSegment = (text: string, speaker: "ME" | "THEY" = "ME"): TranscriptSegment => ({
  id: `p-${speaker}`,
  session_id: "s1",
  speaker,
  text,
  start_ms: 9_000,
  end_ms: 10_000,
  final: false,
});

describe("ChatView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    copyText.mockResolvedValue(true);
  });

  it("mostra a conversa em bolhas com rótulo e tempo de cada fala", () => {
    render(
      <ChatView
        bubbles={[
          speech("a", "ME", "bom dia", 5_000),
          speech("b", "THEY", "bom dia!", 65_000),
        ]}
      />,
    );

    expect(screen.getByText("bom dia")).toBeDefined();
    expect(screen.getByText("Você")).toBeDefined();
    expect(screen.getByText("Interlocutor")).toBeDefined();
    expect(screen.getByText("00:05")).toBeDefined();
    expect(screen.getByText("01:05")).toBeDefined();

    // Alinhamento por falante: você à direita, interlocutor à esquerda.
    expect(screen.getByText("bom dia").closest(".chat__row")?.className).toContain(
      "chat__row--me",
    );
    expect(
      screen.getByText("bom dia!").closest(".chat__row")?.className,
    ).toContain("chat__row--they");
  });

  it("mostra a sugestão da IA na mesma conversa, com rótulo e copiar", async () => {
    render(
      <ChatView
        bubbles={[
          speech("a", "ME", "fala", 1_000),
          suggestion("ai", "Sugira falar do orçamento", 3_000),
        ]}
      />,
    );

    expect(screen.getByText("Sugestão da IA")).toBeDefined();
    const row = screen.getByText("Sugira falar do orçamento").closest(".chat__row");
    expect(row?.className).toContain("chat__row--ai");

    fireEvent.click(screen.getByRole("button", { name: "Copiar Sugestão da IA" }));
    await waitFor(() => {
      expect(copyText).toHaveBeenCalledWith("Sugira falar do orçamento");
    });
    expect(await screen.findByText("Copiado")).toBeDefined();
  });

  it("mostra a fala em andamento no fim, com indicador de digitação", () => {
    render(
      <ChatView
        bubbles={[speech("a", "ME", "fala fechada", 1_000)]}
        pending={[pendingSegment("ainda falando")]}
      />,
    );

    const pendingRow = screen.getByText("ainda falando").closest(".chat__row");
    expect(pendingRow?.className).toContain("chat__row--pending");
    expect(pendingRow?.getAttribute("data-pending")).toBe("true");
    expect(pendingRow?.getAttribute("title")).toContain("transcrevendo");
    // A fala em andamento vem **depois** da linha temporal.
    const rows = screen.getAllByText(/fala fechada|ainda falando/);
    expect(rows[rows.length - 1]?.textContent).toBe("ainda falando");
    expect(screen.getByText("…")).toBeDefined();
  });

  it("mostra o estado vazio e respeita o cabeçalho", () => {
    render(
      <ChatView
        bubbles={[]}
        header={<button type="button">Sugerir</button>}
        emptyLabel="Sem falas"
      />,
    );

    expect(screen.getByRole("button", { name: "Sugerir" })).toBeDefined();
    expect(screen.getByText("Sem falas")).toBeDefined();
  });

  it("renderiza a sugestão da IA como Markdown, com código colorido", () => {
    render(
      <ChatView
        bubbles={[
          {
            kind: "suggestion",
            id: "ai",
            text: "## Solução\n\n```rust\nfn main() {}\n```",
            at_ms: 3_000,
            mode_name: "LeetCode",
          },
        ]}
      />,
    );

    expect(screen.getByText("Sugestão · LeetCode")).toBeDefined();
    expect(screen.getByRole("heading", { name: "Solução" })).toBeDefined();
    expect(screen.getByText("rust")).toBeDefined();
  });
});
