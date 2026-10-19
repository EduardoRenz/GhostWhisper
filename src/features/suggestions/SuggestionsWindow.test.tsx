import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Suggestion } from "../../core/domain/assistant";

const state = vi.hoisted(() => ({
  value: {
    current: null as Suggestion | null,
    quick: null as Suggestion | null,
    history: [] as Suggestion[],
    generating: false,
    loading: false,
    error: null as string | null,
    skipped: false,
    pinned: null as string | null,
    suggestNow: vi.fn(),
    pin: vi.fn(),
  },
}));

vi.mock("../../core/application/usePanelOpen", () => ({
  usePanelOpen: () => true,
}));

vi.mock("../../core/application/useSessionState", () => ({
  useSessionState: () => ({
    state: { listening: true, paused: false, session_id: "s1" },
  }),
}));

vi.mock("../../core/application/useSettings", () => ({
  useSettings: () => ({
    settings: {
      assistant_mode_id: "meeting",
      suggestion_auto_refresh: true,
    },
    reload: vi.fn(),
  }),
}));

vi.mock("../../core/application/useAssistantModes", () => ({
  useAssistantModes: () => ({
    modes: [{ id: "meeting", name: "Reunião" }],
  }),
}));

vi.mock("../../core/application/useSuggestions", () => ({
  useSuggestions: () => state.value,
}));

vi.mock("../../adapters/ipc/settings", () => ({
  saveSettings: vi.fn(async () => []),
}));

vi.mock("../../adapters/ipc/overlay", () => ({
  openSettingsWindow: vi.fn(),
}));

vi.mock("../../core/application/clipboard", () => ({
  copyText: vi.fn(async () => true),
}));

vi.mock("../../components/MarkdownView", () => ({
  MarkdownView: ({ markdown }: { markdown: string }) => (
    <div data-testid="markdown">{markdown}</div>
  ),
}));

vi.mock("../panels/PanelFrame", () => ({
  PanelFrame: ({
    title,
    heading,
    actions,
    children,
  }: {
    title?: string;
    heading?: ReactNode;
    actions?: ReactNode;
    children: ReactNode;
  }) => (
    <div>
      <h1>{heading ?? title}</h1>
      <div>{actions}</div>
      <div className="panel__scroll">{children}</div>
    </div>
  ),
}));

vi.mock("../../core/application/useSuggestionCadence", () => ({
  useSuggestionCadence: () => cadence.value,
}));

const cadence = vi.hoisted(() => ({
  value: {
    enabled: true,
    running: true,
    remaining_ms: 12_000,
    awaiting_speech: false,
    window_ms: 20_000,
  },
}));

import { SuggestionsWindow } from "./SuggestionsWindow";

const suggestion = (overrides: Partial<Suggestion> = {}): Suggestion => ({
  session_id: "s1",
  mode_id: "meeting",
  mode_name: "Reunião",
  mode_icon: "users",
  mode_color: "cyan",
  phase: "full",
  title: "",
  text: "Sugestão atual.",
  at_ms: 1_000,
  context_until_ms: 1_000,
  message_id: null,
  skipped: false,
  ...overrides,
});

describe("SuggestionsWindow — feedback da geração", () => {
  beforeEach(() => {
    state.value = {
      current: null,
      quick: null,
      history: [],
      generating: false,
      loading: false,
      error: null,
      skipped: false,
      pinned: null,
      suggestNow: vi.fn(),
      pin: vi.fn(),
    };
  });

  it("mostra o pill DENTRO do bloco da sugestão ao gerar uma nova", () => {
    state.value.current = suggestion();
    state.value.generating = true;
    const { container } = render(<SuggestionsWindow />);

    const pill = screen.getByRole("status");
    expect(pill.textContent).toContain("Gerando uma nova sugestão");
    // O pill fica no bloco de conteúdo (junto da sugestão), não no cabeçalho.
    expect(container.querySelector(".panel__scroll .status-pill")).not.toBeNull();
    expect(screen.getByTestId("markdown").textContent).toContain(
      "Sugestão atual.",
    );
  });

  it("sem sugestão ainda, o pill diz que está gerando", () => {
    state.value.loading = true;
    state.value.generating = true;
    render(<SuggestionsWindow />);
    expect(screen.getByRole("status").textContent).toContain(
      "Gerando sugestão",
    );
  });

  it("sem geração, não mostra o pill", () => {
    state.value.current = suggestion();
    render(<SuggestionsWindow />);
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("mostra a resposta rápida e a completa em blocos separados (BR-038)", () => {
    state.value.current = suggestion({ text: "resposta completa" });
    state.value.quick = suggestion({
      text: "resposta curta",
      phase: "quick",
      title: "",
    });
    const { container } = render(<SuggestionsWindow />);

    expect(container.querySelector(".suggestion-block--quick")?.textContent).toContain(
      "resposta curta",
    );
    expect(container.querySelector(".suggestion-block--full")?.textContent).toContain(
      "resposta completa",
    );
  });

  it("mostra o cabeçalho novo: faísca, título, LIVE e Suggestar agora (BR-039)", () => {
    state.value.current = suggestion();
    const { container } = render(<SuggestionsWindow />);

    expect(screen.getByRole("heading", { level: 1 }).textContent).toContain(
      "Sugestões",
    );
    expect(container.querySelector(".suggestions-title__spark")).not.toBeNull();
    expect(container.querySelector(".live-badge")?.textContent).toContain("LIVE");
    expect(
      container.querySelector(".suggest-now")?.textContent,
    ).toContain("Sugerir agora");
  });

  it("mostra a contagem regressiva do auto-sugerir", () => {
    state.value.current = suggestion();
    const { container } = render(<SuggestionsWindow />);
    expect(container.querySelector(".auto-control__label")?.textContent).toBe(
      "Auto-sugerir",
    );
    expect(container.querySelector(".auto-control__count")?.textContent).toBe(
      "12s",
    );
  });

  it("mostra só a resposta rápida enquanto a completa não chega", () => {
    state.value.quick = suggestion({ text: "resposta curta", phase: "quick" });
    const { container } = render(<SuggestionsWindow />);

    expect(container.querySelector(".suggestion-block--quick")).not.toBeNull();
    expect(container.querySelector(".suggestion-block--full")).toBeNull();
  });

  it("enquanto a nova monta, a anterior já está no histórico", () => {
    // A nova geração começou (só a rápida): o topo não tem completa e a
    // sugestão anterior aparece no accordion do histórico.
    state.value.current = null;
    state.value.quick = suggestion({
      text: "resposta curta nova",
      phase: "quick",
      title: "",
      at_ms: 2_000,
    });
    state.value.history = [
      suggestion({ text: "## Sugestão anterior", at_ms: 1_000, message_id: "m1" }),
    ];
    const { container } = render(<SuggestionsWindow />);

    expect(container.querySelector(".suggestion-block--full")).toBeNull();
    const historyItem = container.querySelector(".history__item-title");
    expect(historyItem?.textContent).toContain("Sugestão anterior");
  });
});
