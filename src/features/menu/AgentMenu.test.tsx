import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AgentMenu } from "./AgentMenu";
import type { AssistantMode } from "../../core/domain/assistant";

const mocks = vi.hoisted(() => ({
  setAssistantMode: vi.fn(),
  closeMenu: vi.fn(),
  assistantModesList: vi.fn(),
}));

vi.mock("../../adapters/ipc/overlay", () => ({
  closeMenu: mocks.closeMenu,
}));

vi.mock("../../adapters/ipc/settings", () => ({
  setAssistantMode: mocks.setAssistantMode,
}));

vi.mock("../../adapters/ipc/suggestions", () => ({
  assistantModesList: mocks.assistantModesList,
}));

vi.mock("../../core/application/useSettings", () => ({
  useSettings: () => ({
    settings: { assistant_mode_id: "leetcode" },
    reload: vi.fn(),
  }),
}));

const mode = (
  id: string,
  name: string,
  icon: string,
  color: string,
): AssistantMode => ({
  id,
  name,
  category: "engineering",
  system_prompt: "system",
  suggestion_prompt: "suggestion",
  summary_prompt: "summary",
  context: "",
  icon,
  color,
});

const MODES: AssistantMode[] = [
  mode("default", "Padrão", "sparkle", "slate"),
  mode("meeting", "Reunião", "users", "cyan"),
  mode("leetcode", "LeetCode", "code", "emerald"),
];

describe("AgentMenu", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.assistantModesList.mockResolvedValue(MODES);
    mocks.setAssistantMode.mockResolvedValue(undefined);
  });

  it("lista os agentes e marca o ativo", async () => {
    render(<AgentMenu refreshKey={1} />);

    const items = await screen.findAllByRole("menuitemradio");
    expect(items).toHaveLength(3);
    expect(items[2]?.getAttribute("aria-checked")).toBe("true");
  });

  it("escolher troca o agente e fecha o menu", async () => {
    render(<AgentMenu refreshKey={1} />);
    const items = await screen.findAllByRole("menuitemradio");

    fireEvent.click(items[1]!);

    expect(mocks.setAssistantMode).toHaveBeenCalledWith("meeting");
    await waitFor(() => expect(mocks.closeMenu).toHaveBeenCalledTimes(1));
  });

  it("navega com as setas", async () => {
    render(<AgentMenu refreshKey={1} />);
    const items = await screen.findAllByRole("menuitemradio");
    items[0]?.focus();

    fireEvent.keyDown(screen.getByRole("menu"), { key: "ArrowDown" });
    expect(document.activeElement).toBe(items[1]);

    fireEvent.keyDown(screen.getByRole("menu"), { key: "End" });
    expect(document.activeElement).toBe(items[2]);

    fireEvent.keyDown(screen.getByRole("menu"), { key: "Home" });
    expect(document.activeElement).toBe(items[0]);
  });

  it("mostra o vazio quando não há agentes", async () => {
    mocks.assistantModesList.mockResolvedValue([]);
    render(<AgentMenu refreshKey={1} />);

    expect(await screen.findByText("Nenhum agente")).toBeDefined();
  });
});
