import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ChatWindow } from "./ChatWindow";
import { defaultSettings } from "../../core/domain/settings";

const request = vi.fn();
const panelOpen = vi.fn();

vi.mock("../../adapters/ipc/panels", () => ({
  panelOpen: (...args: unknown[]) => panelOpen(...args),
  panelHide: vi.fn(),
  panelIsOpen: vi.fn(async () => false),
  togglePanel: vi.fn(),
}));

vi.mock("../../core/application/usePanelOpen", () => ({
  usePanelOpen: () => true,
}));

vi.mock("../../core/application/useSessionState", () => ({
  useSessionState: () => ({
    state: {
      listening: true,
      paused: false,
      session_id: "sessao-1234",
      channels: ["microphone"],
      started_at_ms: 0,
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

vi.mock("../../core/application/useLiveTranscription", () => ({
  useLiveTranscription: () => ({
    segments: [
      {
        id: "t1",
        session_id: "sessao-1234",
        speaker: "THEY",
        text: "tudo certo com o prazo?",
        start_ms: 0,
        end_ms: 1_000,
        final: true,
      },
    ],
  }),
}));

vi.mock("../../core/application/useInsight", () => ({
  useInsight: () => ({
    suggestions: [],
    latest: null,
    loading: false,
    error: null,
    request,
  }),
}));

vi.mock("../../core/application/useSettings", () => ({
  useSettings: () => ({ settings: defaultSettings(), reload: vi.fn() }),
}));

vi.mock("../../core/application/useTranslation", () => ({
  useTranslation: () => ({
    translations: {},
    pending: {},
    translate: vi.fn(),
    reset: vi.fn(),
  }),
}));

describe("ChatWindow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("mostra a conversa da sessão numa moldura de painel", () => {
    const { container } = render(<ChatWindow />);

    expect(screen.getByText("Conversa")).toBeDefined();
    expect(screen.getByText("tudo certo com o prazo?")).toBeDefined();
    // Mesma casca das outras janelas (alças de resize).
    expect(container.querySelectorAll(".panel__resize")).toHaveLength(8);
  });

  it("pede sugestão pelo cabeçalho e abre Resumo/Sugestões", () => {
    render(<ChatWindow />);

    fireEvent.click(screen.getByRole("button", { name: "Sugerir" }));
    expect(request).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Resumo" }));
    expect(panelOpen).toHaveBeenCalledWith("summary");

    fireEvent.click(screen.getByRole("button", { name: "Sugestões" }));
    expect(panelOpen).toHaveBeenCalledWith("suggestions");
  });
});
