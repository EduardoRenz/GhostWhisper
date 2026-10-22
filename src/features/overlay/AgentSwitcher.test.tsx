import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AgentSwitcher } from "./AgentSwitcher";
import type { AssistantMode } from "../../core/domain/assistant";

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

const button = (name: string) =>
  screen.getByRole("button", { name: `Agente: ${name}` });

describe("AgentSwitcher", () => {
  it("mostra o agente ativo no pill compacto", () => {
    const { container } = render(
      <AgentSwitcher modes={MODES} modeId="leetcode" onSelect={() => {}} />,
    );

    const trigger = button("LeetCode");
    expect(trigger.textContent).toContain("LeetCode");
    expect(trigger.textContent).toContain("Alternar agente");
    expect(
      container.querySelector(".agent-switcher")?.getAttribute("style"),
    ).toContain("--oh-agent-emerald");
  });

  it("abre o menu e lista os agentes", () => {
    render(<AgentSwitcher modes={MODES} modeId="leetcode" onSelect={() => {}} />);

    fireEvent.click(button("LeetCode"));

    const items = screen.getAllByRole("menuitemradio");
    expect(items).toHaveLength(3);
    expect(items[0]?.textContent).toContain("Padrão");
    expect(items[1]?.textContent).toContain("Reunião");
    expect(items[2]?.textContent).toContain("LeetCode");
    expect(items[2]?.getAttribute("aria-checked")).toBe("true");
  });

  it("escolhe um agente e fecha o menu", () => {
    const onSelect = vi.fn();
    render(<AgentSwitcher modes={MODES} modeId="leetcode" onSelect={onSelect} />);

    fireEvent.click(button("LeetCode"));
    fireEvent.click(screen.getByRole("menuitemradio", { name: /Reunião/ }));

    expect(onSelect).toHaveBeenCalledWith("meeting");
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("fecha com Escape", () => {
    render(<AgentSwitcher modes={MODES} modeId="leetcode" onSelect={() => {}} />);
    fireEvent.click(button("LeetCode"));

    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("navega pelo menu com as setas", () => {
    render(<AgentSwitcher modes={MODES} modeId="leetcode" onSelect={() => {}} />);
    fireEvent.click(button("LeetCode"));

    const menu = screen.getByRole("menu");
    const items = screen.getAllByRole("menuitemradio");
    items[0]?.focus();
    fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(document.activeElement).toBe(items[1]);

    fireEvent.keyDown(menu, { key: "End" });
    expect(document.activeElement).toBe(items[2]);

    fireEvent.keyDown(menu, { key: "Home" });
    expect(document.activeElement).toBe(items[0]);
  });

  it("avisa o overlay quando o menu abre e fecha", () => {
    const onOpenChange = vi.fn();
    render(
      <AgentSwitcher
        modes={MODES}
        modeId="leetcode"
        onSelect={() => {}}
        onOpenChange={onOpenChange}
      />,
    );

    fireEvent.click(button("LeetCode"));
    expect(onOpenChange).toHaveBeenLastCalledWith(true);

    fireEvent.click(button("LeetCode"));
    expect(onOpenChange).toHaveBeenLastCalledWith(false);
  });

  it("usa o nome do Padrão quando o catálogo ainda não carregou", () => {
    render(<AgentSwitcher modes={[]} modeId="default" onSelect={() => {}} />);
    expect(button("Padrão")).toBeDefined();
  });

  it("mostra o vazio quando não há agentes", () => {
    render(<AgentSwitcher modes={[]} modeId="default" onSelect={() => {}} />);
    fireEvent.click(button("Padrão"));
    expect(screen.getByText("Nenhum agente")).toBeDefined();
  });
});
