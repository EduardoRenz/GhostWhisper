import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { HistoryPanel } from "./HistoryPanel";
import type { SessionSummary, TimelineEntry } from "../../core/domain/transcript";

const listSessions = vi.fn();
const sessionTimeline = vi.fn();
const renameSession = vi.fn();
const deleteSession = vi.fn();
const exportSessionMarkdown = vi.fn();
const saveSessionMarkdown = vi.fn();
const pickMarkdownPath = vi.fn();
const copyText = vi.fn();

vi.mock("../../adapters/ipc/database", () => ({
  listSessions: (...args: unknown[]) => listSessions(...args),
  sessionTimeline: (...args: unknown[]) => sessionTimeline(...args),
  renameSession: (...args: unknown[]) => renameSession(...args),
  deleteSession: (...args: unknown[]) => deleteSession(...args),
}));

vi.mock("../../adapters/ipc/export", () => ({
  exportSessionMarkdown: (...args: unknown[]) => exportSessionMarkdown(...args),
  saveSessionMarkdown: (...args: unknown[]) => saveSessionMarkdown(...args),
  pickMarkdownPath: (...args: unknown[]) => pickMarkdownPath(...args),
}));

vi.mock("../../core/application/clipboard", () => ({
  copyText: (...args: unknown[]) => copyText(...args),
}));

const STARTED = new Date(2026, 0, 2, 3, 4).getTime();

const sessions: SessionSummary[] = [
  {
    id: "s2",
    started_at_ms: STARTED + 3_600_000,
    ended_at_ms: STARTED + 3_660_000,
    subject: "Reunião de sexta",
    status: "ended",
  },
  {
    id: "s1",
    started_at_ms: STARTED,
    ended_at_ms: STARTED + 65_000,
    subject: null,
    status: "ended",
  },
];

const timeline: TimelineEntry[] = [
  { at_ms: 0, kind: "speech", speaker: "THEY", text: "podemos fechar?" },
  { at_ms: 10_000, kind: "suggestion", text: "Confirme o prazo." },
  { at_ms: 30_000, kind: "speech", speaker: "ME", text: "vou enviar hoje" },
];

describe("HistoryPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    listSessions.mockResolvedValue(sessions);
    sessionTimeline.mockResolvedValue(timeline);
    renameSession.mockResolvedValue(undefined);
    deleteSession.mockResolvedValue(undefined);
    exportSessionMarkdown.mockResolvedValue("# Reunião");
    saveSessionMarkdown.mockResolvedValue("/tmp/ghostwhisper.md");
    pickMarkdownPath.mockResolvedValue("/tmp/ghostwhisper.md");
    copyText.mockResolvedValue(true);
  });

  it("lista as sessões com título, data e duração", async () => {
    render(<HistoryPanel activeSessionId={null} onContinue={vi.fn()} />);

    expect(await screen.findByText("Reunião de sexta")).toBeDefined();
    expect(screen.getByText("Sessão sem título")).toBeDefined();
    expect(screen.getByText(/02\/01\/2026 03:04 · 01:05/)).toBeDefined();
  });

  it("abre o detalhe com falas e sugestões marcadas com o tempo", async () => {
    render(<HistoryPanel activeSessionId={null} onContinue={vi.fn()} />);

    fireEvent.click(await screen.findByText("Reunião de sexta"));

    await waitFor(() => {
      expect(screen.getByText("podemos fechar?")).toBeDefined();
    });
    expect(screen.getByText("Confirme o prazo.")).toBeDefined();
    // Sugestão e falas são bolhas na mesma linha temporal, cada uma com seu tempo.
    expect(screen.getByText("Sugestão da IA")).toBeDefined();
    expect(screen.getByText("00:00")).toBeDefined();
    expect(screen.getByText("00:10")).toBeDefined();
    expect(screen.getByText("00:30")).toBeDefined();
    expect(
      screen.getByText("Confirme o prazo.").closest(".chat__row")?.className,
    ).toContain("chat__row--ai");
  });

  it("continua uma sessão encerrada", async () => {
    const onContinue = vi.fn();
    render(<HistoryPanel activeSessionId={null} onContinue={onContinue} />);

    const rows = await screen.findAllByRole("button", { name: "Continuar" });
    fireEvent.click(rows[0]!);

    expect(onContinue).toHaveBeenCalledWith("s2");
  });

  it("renomeia pelo detalhe", async () => {
    render(<HistoryPanel activeSessionId={null} onContinue={vi.fn()} />);
    fireEvent.click(await screen.findByText("Sessão sem título"));

    fireEvent.click(await screen.findByRole("button", { name: "Renomear" }));
    fireEvent.change(screen.getByLabelText("Título da sessão"), {
      target: { value: "Planejamento" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Salvar" }));

    await waitFor(() => {
      expect(renameSession).toHaveBeenCalledWith("s1", "Planejamento");
    });
  });

  it("exporta em Markdown pelo caminho escolhido no diálogo", async () => {
    render(<HistoryPanel activeSessionId={null} onContinue={vi.fn()} />);
    fireEvent.click(await screen.findByText("Reunião de sexta"));

    fireEvent.click(await screen.findByRole("button", { name: "Exportar .md" }));

    await waitFor(() => {
      expect(pickMarkdownPath).toHaveBeenCalled();
    });
    expect(saveSessionMarkdown).toHaveBeenCalledWith(
      "s2",
      "/tmp/ghostwhisper.md",
    );
    // O nome sugerido carrega a data e o título da sessão.
    expect(pickMarkdownPath.mock.calls[0]?.[0]).toMatch(
      /^ghostwhisper-\d{8}-\d{4}-reuniao-de-sexta$/,
    );
    expect(await screen.findByText(/Exportado: \/tmp\/ghostwhisper\.md/)).toBeDefined();
  });

  it("não salva quando o usuário cancela o diálogo", async () => {
    pickMarkdownPath.mockResolvedValue(null);
    render(<HistoryPanel activeSessionId={null} onContinue={vi.fn()} />);
    fireEvent.click(await screen.findByText("Reunião de sexta"));

    fireEvent.click(await screen.findByRole("button", { name: "Exportar .md" }));

    await waitFor(() => {
      expect(pickMarkdownPath).toHaveBeenCalled();
    });
    expect(saveSessionMarkdown).not.toHaveBeenCalled();
  });

  it("copia o Markdown da sessão", async () => {
    render(<HistoryPanel activeSessionId={null} onContinue={vi.fn()} />);
    fireEvent.click(await screen.findByText("Reunião de sexta"));

    fireEvent.click(await screen.findByRole("button", { name: "Copiar Markdown" }));

    await waitFor(() => {
      expect(copyText).toHaveBeenCalledWith("# Reunião");
    });
    expect(await screen.findByText("Markdown copiado.")).toBeDefined();
  });

  it("exclui a sessão", async () => {
    render(<HistoryPanel activeSessionId={null} onContinue={vi.fn()} />);
    fireEvent.click(await screen.findByText("Reunião de sexta"));

    fireEvent.click(await screen.findByRole("button", { name: "Excluir" }));

    await waitFor(() => {
      expect(deleteSession).toHaveBeenCalledWith("s2");
    });
  });

  it("avisa quando não há sessões", async () => {
    listSessions.mockResolvedValue([]);
    render(<HistoryPanel activeSessionId={null} onContinue={vi.fn()} />);

    expect(await screen.findByText(/Nenhuma sessão ainda/)).toBeDefined();
  });
});
