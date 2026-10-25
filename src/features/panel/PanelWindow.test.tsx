import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const saveSettings = vi.fn().mockResolvedValue([]);
const reload = vi.fn().mockResolvedValue(undefined);

vi.mock("../../adapters/ipc/panels", () => ({
  panelHide: vi.fn(),
  panelIsOpen: vi.fn(async () => false),
  togglePanel: vi.fn(),
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: vi.fn(async () => () => {}),
}));

vi.mock("../../adapters/ipc/settings", () => ({
  saveSettings: (...args: unknown[]) => saveSettings(...args),
}));

vi.mock("../../core/application/usePanelOpen", () => ({
  usePanelOpen: () => true,
}));

vi.mock("../../core/application/useSettings", () => ({
  useSettings: () => ({
    settings: { panel_tab: "transcription" },
    reload,
  }),
}));

vi.mock("../../core/application/useSessionState", () => ({
  useSessionState: () => ({
    state: {
      listening: false,
      paused: false,
      session_id: null,
      channels: [],
      started_at_ms: null,
    },
  }),
}));

vi.mock("./TranscriptionView", () => ({
  TranscriptionView: () => <div>conversa</div>,
}));

vi.mock("./SummaryView", () => ({
  SummaryView: ({ active }: { opened: boolean; active: boolean }) => (
    <div>resumo {active ? "ativo" : "inativo"}</div>
  ),
}));

vi.mock("../journal/JournalView", () => ({
  JournalView: () => <div>diário</div>,
}));

import { PanelWindow } from "./PanelWindow";

const tab = (name: string) => screen.getByRole("tab", { name });

describe("PanelWindow (painel da sessão em abas)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("abre na Transcrição por padrão", () => {
    render(<PanelWindow />);
    expect(tab("Transcrição").getAttribute("aria-selected")).toBe("true");
    expect(tab("Resumo").getAttribute("aria-selected")).toBe("false");
  });

  it("tem a aba Sessão (o diário de perguntas)", () => {
    render(<PanelWindow />);
    expect(tab("Sessão").getAttribute("aria-selected")).toBe("false");
  });

  it("troca de aba ao clicar e persiste a escolha", async () => {
    render(<PanelWindow />);
    fireEvent.click(tab("Resumo"));

    expect(tab("Resumo").getAttribute("aria-selected")).toBe("true");
    expect(saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({ panel_tab: "summary" }),
    );
    await vi.waitFor(() => expect(reload).toHaveBeenCalled());
  });
});
