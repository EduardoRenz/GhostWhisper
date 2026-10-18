import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SummaryWindow } from "./SummaryWindow";
import type { MeetingSummary } from "../../core/domain/assistant";
import { defaultSettings } from "../../core/domain/settings";

const useSummaryMock = vi.fn();
const select = vi.fn();
const showLatest = vi.fn();
const regenerate = vi.fn();

vi.mock("../../core/application/usePanelOpen", () => ({
  usePanelOpen: () => true,
}));

vi.mock("../../core/application/useSessionState", () => ({
  useSessionState: () => ({
    state: {
      listening: true,
      paused: false,
      session_id: "s1",
      channels: [],
      started_at_ms: 1_000,
    },
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

vi.mock("../../core/application/useSettings", () => ({
  useSettings: () => ({ settings: defaultSettings(), reload: vi.fn() }),
}));

vi.mock("../../core/application/useSummary", () => ({
  useSummary: (...args: unknown[]) => useSummaryMock(...args),
}));

vi.mock("../../adapters/ipc/overlay", () => ({
  openSettingsWindow: vi.fn(),
}));

const version = (
  subject: string,
  markdown: string,
  updatedAt: number,
): MeetingSummary => ({
  session_id: "s1",
  subject,
  markdown,
  summarized_from_ms: 0,
  summarized_until_ms: 30_000,
  updated_at_ms: updatedAt,
});

describe("SummaryWindow", () => {
  const current = version("Planejamento", "## Visão geral\n\natual", 2_000);
  const older = version("Abertura", "## Visão geral\n\nantiga", 1_000);

  beforeEach(() => {
    vi.clearAllMocks();
    useSummaryMock.mockReturnValue({
      summary: current,
      history: [current, older],
      viewing: null,
      loading: false,
      error: null,
      hasMore: false,
      refresh: vi.fn(),
      regenerate,
      select,
      showLatest,
    });
  });

  it("mostra o tópico no subtítulo e lista as versões anteriores", () => {
    render(<SummaryWindow />);

    expect(screen.getByText("Planejamento · Gravando")).toBeDefined();
    expect(screen.getByText("Resumos anteriores (1)")).toBeDefined();
    expect(screen.getByText("Abertura")).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: /Abertura/ }));
    expect(select).toHaveBeenCalledWith(older);
  });

  it("congela em uma versão antiga e volta ao atual", () => {
    useSummaryMock.mockReturnValue({
      summary: current,
      history: [current, older],
      viewing: older,
      loading: false,
      error: null,
      hasMore: false,
      refresh: vi.fn(),
      regenerate,
      select,
      showLatest,
    });

    render(<SummaryWindow />);

    expect(screen.getByText(/Vendo versão de/)).toBeDefined();
    expect(screen.getByText("Abertura · Gravando")).toBeDefined();
    expect(screen.getByText("antiga")).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Voltar ao atual" }));
    expect(showLatest).toHaveBeenCalled();
  });

  it("não lista versões quando só existe a atual", () => {
    useSummaryMock.mockReturnValue({
      summary: current,
      history: [current],
      viewing: null,
      loading: false,
      error: null,
      hasMore: false,
      refresh: vi.fn(),
      regenerate,
      select,
      showLatest,
    });

    render(<SummaryWindow />);

    expect(screen.queryByText(/Resumos anteriores/)).toBeNull();
  });
});
