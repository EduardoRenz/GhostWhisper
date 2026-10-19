import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Suggestion } from "../../core/domain/assistant";

const state = vi.hoisted(() => ({
  value: {
    current: null as Suggestion | null,
    history: [] as Suggestion[],
    generating: false,
    loading: false,
    error: null as string | null,
    skipped: false,
    pinned: null as string | null,
    suggestNow: vi.fn(),
    pin: vi.fn(),
    select: vi.fn(),
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
    actions,
    children,
  }: {
    title: string;
    actions?: ReactNode;
    children: ReactNode;
  }) => (
    <div>
      <h1>{title}</h1>
      <div>{actions}</div>
      <div className="panel__scroll">{children}</div>
    </div>
  ),
}));

import { SuggestionsWindow } from "./SuggestionsWindow";

const suggestion = (overrides: Partial<Suggestion> = {}): Suggestion => ({
  session_id: "s1",
  mode_id: "meeting",
  mode_name: "Reunião",
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
      history: [],
      generating: false,
      loading: false,
      error: null,
      skipped: false,
      pinned: null,
      suggestNow: vi.fn(),
      pin: vi.fn(),
      select: vi.fn(),
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
});
