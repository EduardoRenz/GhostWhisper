import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ChatView } from "./ChatView";
import { buildChatTimeline, type ChatBubble } from "../../core/domain/chat";
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

const segment = (
  overrides: Partial<TranscriptSegment> & { id: string },
): TranscriptSegment => ({
  session_id: "s1",
  speaker: "ME",
  text: "fala",
  start_ms: 1_000,
  end_ms: 2_000,
  final: true,
  ...overrides,
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
    expect(screen.getByText("Microfone")).toBeDefined();
    expect(screen.getByText("Outros")).toBeDefined();
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
    expect(screen.getByText("Falando agora…")).toBeDefined();
    expect(screen.getByText("Streaming")).toBeDefined();
  });

  it("divide fala longa em continuações, sem repetir rótulo nem tempo", () => {
    const long = `${"a".repeat(100)}. ${"b".repeat(100)}.`;
    // Mesmo caminho da tela: a linha temporal corta a fala antes da renderização.
    const { bubbles } = buildChatTimeline([
      segment({ id: "t1", speaker: "THEY", start_ms: 5_000, text: long }),
    ]);
    render(<ChatView bubbles={bubbles} />);

    const first = screen.getByText(`${"a".repeat(100)}.`);
    const second = screen.getByText(`${"b".repeat(100)}.`);
    expect(first.closest(".chat__row")?.className).toContain("chat__row--they");
    expect(second.closest(".chat__row")?.className).toContain(
      "chat__row--continues",
    );
    // Rótulo e tempo só na primeira parte.
    expect(screen.getAllByText("Outros")).toHaveLength(1);
    expect(screen.getAllByText("00:05")).toHaveLength(1);
  });

  it("corta a prévia longa em continuações, com o `…` só na última", () => {
    const long = `${"a".repeat(100)}. ${"b".repeat(100)}.`;
    render(<ChatView bubbles={[]} pending={[pendingSegment(long, "THEY")]} />);

    expect(screen.getAllByText("Streaming")).toHaveLength(1);
    const rows = screen.getAllByText(/^a+\.$|^b+\.$/);
    expect(rows).toHaveLength(2);
    expect(rows[1]?.closest(".chat__row")?.className).toContain(
      "chat__row--continues",
    );
  });

  it("mostra o estado vazio", () => {
    render(<ChatView bubbles={[]} emptyLabel="Sem falas" />);

    expect(screen.getByText("Sem falas")).toBeDefined();
  });

  it("aciona a IA com o texto inteiro do segmento e só na fala finalizada", () => {
    const onAskAi = vi.fn();
    const long = `${"a".repeat(100)}. ${"b".repeat(100)}.`;
    const { bubbles } = buildChatTimeline([
      segment({ id: "t1", speaker: "THEY", start_ms: 5_000, text: long }),
    ]);
    render(
      <ChatView
        bubbles={bubbles}
        pending={[pendingSegment("prévia")]}
        onAskAi={onAskAi}
      />,
    );

    const buttons = screen.getAllByRole("button", {
      name: "Sugerir com IA a partir desta mensagem",
    });
    expect(buttons).toHaveLength(1);
    fireEvent.click(buttons[0]!);
    expect(onAskAi).toHaveBeenCalledWith({
      id: "t1",
      speaker: "THEY",
      text: long,
      atMs: 5_000,
    });
  });

  it("pede tradução ao clicar e alterna a linha traduzida na bolha", () => {
    const onTranslate = vi.fn();
    const props = {
      bubbles: [speech("a", "ME", "bom dia", 1_000)],
      translationEnabled: true,
      translationTarget: "en",
      onTranslate,
    };
    const { rerender } = render(<ChatView {...props} translations={{}} />);

    fireEvent.click(screen.getByRole("button", { name: "Traduzir para en" }));
    expect(onTranslate).toHaveBeenCalledWith({ id: "a", text: "bom dia" });

    rerender(<ChatView {...props} translations={{ a: "good morning" }} />);
    expect(screen.getByText("good morning")).toBeDefined();
    // A tradução fica abaixo, com separador tracejado.
    expect(
      screen.getByText("good morning").className,
    ).toContain("chat__translation");

    fireEvent.click(screen.getByRole("button", { name: "Ocultar tradução" }));
    expect(screen.queryByText("good morning")).toBeNull();
  });

  it("não mostra o botão de traduzir quando a tradução está desligada", () => {
    render(<ChatView bubbles={[speech("a", "ME", "bom dia", 1_000)]} />);
    expect(screen.queryByRole("button", { name: /Traduzir/ })).toBeNull();
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
