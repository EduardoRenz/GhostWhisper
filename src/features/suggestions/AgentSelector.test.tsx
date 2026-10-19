import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AssistantMode } from "../../core/domain/assistant";
import { AgentSelector } from "./AgentSelector";

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
  context: "",
  icon,
  color,
});

const MODES: AssistantMode[] = [
  mode("meeting", "Reunião", "users", "cyan"),
  mode("leetcode", "LeetCode", "code", "emerald"),
];

describe("AgentSelector", () => {
  it("mostra o agente selecionado com ícone, nome e pill ATIVO", () => {
    const { container } = render(
      <AgentSelector modes={MODES} modeId="leetcode" onSelect={() => {}} />,
    );
    const button = container.querySelector(".agent-selector__button");
    expect(button?.textContent).toContain("LeetCode");
    expect(button?.textContent).toContain("ATIVO");
    expect(container.querySelector(".agent-selector__icon")).not.toBeNull();
    // A cor do agente vem do token do design system.
    expect(button?.getAttribute("style")).toContain("--oh-agent-emerald");
  });

  it("abre o popover com todos os agentes (ícone + nome)", () => {
    render(<AgentSelector modes={MODES} modeId="meeting" onSelect={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /Agente/ }));

    const items = screen.getAllByRole("menuitemradio");
    expect(items).toHaveLength(2);
    expect(items[0]?.textContent).toContain("Reunião");
    expect(items[1]?.textContent).toContain("LeetCode");
    expect(items[0]?.getAttribute("aria-checked")).toBe("true");
  });

  it("escolhe um agente e fecha o popover", () => {
    const onSelect = vi.fn();
    render(<AgentSelector modes={MODES} modeId="meeting" onSelect={onSelect} />);
    fireEvent.click(screen.getByRole("button", { name: /Agente/ }));
    fireEvent.click(screen.getByRole("menuitemradio", { name: /LeetCode/ }));

    expect(onSelect).toHaveBeenCalledWith("leetcode");
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("fecha com Escape", () => {
    render(<AgentSelector modes={MODES} modeId="meeting" onSelect={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /Agente/ }));
    expect(screen.queryByRole("menu")).not.toBeNull();

    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("navega com as setas", () => {
    render(<AgentSelector modes={MODES} modeId="meeting" onSelect={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /Agente/ }));

    const items = screen.getAllByRole("menuitemradio");
    items[0]?.focus();
    fireEvent.keyDown(screen.getByRole("menu"), { key: "ArrowDown" });
    expect(document.activeElement).toBe(items[1]);
  });
});
