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

const llmSource = vi.hoisted(() => ({
  value: null as null | {
    source_id: string;
    label: string;
    provider: string;
    model: string;
    scope: string;
    is_fallback: boolean;
    latency_ms: number;
  },
}));

vi.mock("../../core/application/useLlmSource", () => ({
  useLlmSource: () => llmSource.value,
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
    llmSource.value = null;
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

  it("mostra o modelo que respondeu o resumo (BR-051)", () => {
    state.value.summary = summary();
    llmSource.value = {
      source_id: "fonte-1",
      label: "DeepSeek",
      provider: "deepseek",
      model: "deepseek-reasoner",
      scope: "capability:summary",
      is_fallback: false,
      latency_ms: 142,
    };
    render(<SummaryView opened active />);

    expect(screen.getByText(/DeepSeek · deepseek-reasoner/)).toBeDefined();
  });

  it("marca o resumo que veio de uma fonte em fallback", () => {
    state.value.summary = summary();
    llmSource.value = {
      source_id: "fonte-2",
      label: "OpenAI",
      provider: "openai",
      model: "gpt-4o-mini",
      scope: "capability:summary",
      is_fallback: true,
      latency_ms: 90,
    };
    render(<SummaryView opened active />);

    expect(screen.getByText(/\(fallback\)/)).toBeDefined();
  });
});
