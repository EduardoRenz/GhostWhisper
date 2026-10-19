import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { MeetingSummary } from "../../core/domain/assistant";

const state = vi.hoisted(() => ({
  value: {
    summary: null as MeetingSummary | null,
    history: [] as MeetingSummary[],
    viewing: null as MeetingSummary | null,
    loading: false,
    error: null as string | null,
    hasMore: false,
    regenerate: vi.fn(),
    select: vi.fn(),
    showLatest: vi.fn(),
  },
}));

vi.mock("../../core/application/useSummary", () => ({
  useSummary: () => state.value,
}));

vi.mock("../../core/application/useSessionState", () => ({
  useSessionState: () => ({
    state: {
      session_id: "s1",
      listening: true,
      paused: false,
      channels: [],
      started_at_ms: null,
    },
  }),
}));

vi.mock("../../core/application/useSettings", () => ({
  useSettings: () => ({
    settings: { summary_auto_refresh: true, summary_interval_secs: 30 },
  }),
}));

vi.mock("../../core/application/clipboard", () => ({
  copyText: vi.fn(async () => true),
}));

vi.mock("../../adapters/ipc/overlay", () => ({
  openSettingsWindow: vi.fn(),
}));

vi.mock("../../components/MarkdownView", () => ({
  MarkdownView: ({ markdown }: { markdown: string }) => (
    <div data-testid="markdown">{markdown}</div>
  ),
}));

import { SummaryView } from "./SummaryView";

const summary = (overrides: Partial<MeetingSummary> = {}): MeetingSummary => ({
  session_id: "s1",
  subject: "Planejamento do trimestre",
  markdown: "## Visão geral\n\nConteúdo.",
  summarized_from_ms: 0,
  summarized_until_ms: 1_000,
  updated_at_ms: 1_000,
  ...overrides,
});

describe("SummaryView — feedback da geração", () => {
  beforeEach(() => {
    state.value = {
      summary: null,
      history: [],
      viewing: null,
      loading: false,
      error: null,
      hasMore: false,
      regenerate: vi.fn(),
      select: vi.fn(),
      showLatest: vi.fn(),
    };
  });

  it("mostra 'Gerando resumo completo…' enquanto gera sem resumo anterior", () => {
    state.value.loading = true;
    render(<SummaryView opened active />);
    expect(screen.getByRole("status").textContent).toContain(
      "Gerando resumo completo",
    );
  });

  it("mostra o título e avisa que o restante está sendo processado", () => {
    state.value.summary = summary();
    state.value.hasMore = true;
    render(<SummaryView opened active />);

    expect(
      screen.getByRole("heading", { name: "Planejamento do trimestre" }),
    ).toBeDefined();
    expect(screen.getByRole("status").textContent).toContain(
      "Processando o restante",
    );
    expect(screen.getByText(/Resumo parcial/)).toBeDefined();
  });

  it("sem pendências, marca o resumo como completo", () => {
    state.value.summary = summary();
    render(<SummaryView opened active />);

    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.getByText("Planejamento do trimestre")).toBeDefined();
    expect(screen.getByText(/Resumo completo/)).toBeDefined();
  });
});
