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

describe("AgentSwitcher (pill)", () => {
  it("mostra o agente ativo no pill compacto", () => {
    const { container } = render(
      <AgentSwitcher modes={MODES} modeId="leetcode" />,
    );

    const trigger = button("LeetCode");
    expect(trigger.textContent).toContain("LeetCode");
    expect(trigger.textContent).toContain("Alternar agente");
    expect(
      container.querySelector(".agent-switcher")?.getAttribute("style"),
    ).toContain("--oh-agent-emerald");
  });

  it("abre o menu flutuante com a âncora do botão", () => {
    const onOpenMenu = vi.fn();
    render(
      <AgentSwitcher
        modes={MODES}
        modeId="leetcode"
        onOpenMenu={onOpenMenu}
      />,
    );

    fireEvent.click(button("LeetCode"));

    expect(onOpenMenu).toHaveBeenCalledTimes(1);
    const anchor = onOpenMenu.mock.calls[0]?.[0] as DOMRect;
    expect(anchor.width).toBeGreaterThanOrEqual(0);
    expect(typeof anchor.x).toBe("number");
  });

  it("usa o nome do Padrão quando o catálogo ainda não carregou", () => {
    render(<AgentSwitcher modes={[]} modeId="default" />);
    expect(button("Padrão")).toBeDefined();
  });
});
