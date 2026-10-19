import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { HistoryWindow } from "./HistoryWindow";
import { ALL_CHANNELS } from "../../core/domain/transcript";

const continueSession = vi.fn();
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
      listening: false,
      paused: false,
      session_id: null,
      channels: [],
      started_at_ms: null,
    },
    busy: false,
    error: null,
    start: vi.fn(),
    pause: vi.fn(),
    resume: vi.fn(),
    stop: vi.fn(),
    continueSession: (...args: unknown[]) => continueSession(...args),
    setChannels: vi.fn(),
  }),
}));

vi.mock("./HistoryPanel", () => ({
  HistoryPanel: ({ onContinue }: { onContinue: (id: string) => void }) => (
    <button type="button" onClick={() => onContinue("s1")}>
      abrir s1
    </button>
  ),
}));

describe("HistoryWindow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("embrulha o histórico na moldura de painel", () => {
    const { container } = render(<HistoryWindow />);

    expect(screen.getByText("Histórico")).toBeDefined();
    expect(container.querySelectorAll(".panel__resize")).toHaveLength(8);
  });

  it("continuar uma sessão volta a capturar e traz a janela de Chat", () => {
    render(<HistoryWindow />);

    fireEvent.click(screen.getByRole("button", { name: "abrir s1" }));

    expect(continueSession).toHaveBeenCalledWith("s1", ALL_CHANNELS);
    expect(panelOpen).toHaveBeenCalledWith("chat");
  });
});
