import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { InsightsPanel } from "./InsightsPanel";
import type { InsightSuggestion } from "../../core/domain/transcript";

const sessionMessages = vi.fn();
const llmInsight = vi.fn();
const copyText = vi.fn();

vi.mock("../../adapters/ipc/database", () => ({
  sessionMessages: (...args: unknown[]) => sessionMessages(...args),
}));

vi.mock("../../adapters/ipc/llm", () => ({
  llmInsight: (...args: unknown[]) => llmInsight(...args),
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: async () => () => {},
}));

vi.mock("../../core/application/clipboard", () => ({
  copyText: (...args: unknown[]) => copyText(...args),
}));

const stored = {
  id: "m1",
  session_id: "s1",
  role: "assistant" as const,
  content: "Confirme o prazo com o time.",
  timestamp_ms: 10_000,
};

const created: InsightSuggestion = {
  message_id: "m2",
  session_id: "s1",
  text: "Pergunte sobre o orçamento.",
  at_ms: 65_000,
};

describe("InsightsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sessionMessages.mockResolvedValue([stored]);
    llmInsight.mockResolvedValue(created);
    copyText.mockResolvedValue(true);
  });

  it("mostra as sugestões gravadas com o tempo de cada uma", async () => {
    render(<InsightsPanel sessionId="s1" />);

    expect(
      await screen.findByText("Confirme o prazo com o time."),
    ).toBeDefined();
    expect(screen.getByText(/🤖 00:10/)).toBeDefined();
  });

  it("pede uma nova sugestão e a acrescenta à linha temporal", async () => {
    render(<InsightsPanel sessionId="s1" />);
    await screen.findByText("Confirme o prazo com o time.");

    fireEvent.click(screen.getByRole("button", { name: "Sugerir" }));

    await waitFor(() => {
      expect(llmInsight).toHaveBeenCalledWith("s1");
    });
    expect(await screen.findByText("Pergunte sobre o orçamento.")).toBeDefined();
    expect(screen.getByText(/🤖 01:05/)).toBeDefined();
  });

  it("copia o texto da sugestão", async () => {
    render(<InsightsPanel sessionId="s1" />);
    await screen.findByText("Confirme o prazo com o time.");

    fireEvent.click(screen.getByRole("button", { name: "Copiar" }));

    await waitFor(() => {
      expect(copyText).toHaveBeenCalledWith("Confirme o prazo com o time.");
    });
    expect(await screen.findByRole("button", { name: "Copiado" })).toBeDefined();
  });

  it("mostra a sugestão empurrada pelo atalho sem duplicar", async () => {
    render(<InsightsPanel sessionId="s1" externalSuggestion={created} />);

    await waitFor(() => {
      expect(screen.getByText("Pergunte sobre o orçamento.")).toBeDefined();
    });
    expect(screen.getByText("Confirme o prazo com o time.")).toBeDefined();
  });

  it("explica quando ainda não há sugestão", async () => {
    sessionMessages.mockResolvedValue([]);
    render(<InsightsPanel sessionId="s1" />);

    expect(await screen.findByText(/Nenhuma sugestão ainda/)).toBeDefined();
  });
});
