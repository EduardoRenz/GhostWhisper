import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AGENT_ICON_IDS } from "../core/domain/assistant";
import { AgentBadge } from "./AgentBadge";
import { AgentIcon } from "./AgentIcon";

describe("AgentIcon", () => {
  it("desenha um svg decorativo para cada ícone do catálogo", () => {
    for (const icon of AGENT_ICON_IDS) {
      const { container, unmount } = render(<AgentIcon icon={icon} />);
      const svg = container.querySelector("svg");
      expect(svg, icon).not.toBeNull();
      expect(svg?.getAttribute("aria-hidden"), icon).toBe("true");
      expect(svg?.getAttribute("viewBox"), icon).toBe("0 0 24 24");
      unmount();
    }
  });

  it("cai para a faísca quando o ícone é desconhecido", () => {
    const { container } = render(<AgentIcon icon="rocket" />);
    const svg = container.querySelector("svg");
    const fallback = render(<AgentIcon icon="sparkle" />);
    expect(svg?.innerHTML).toBe(fallback.container.querySelector("svg")?.innerHTML);
  });

  it("respeita o tamanho informado", () => {
    const { container } = render(<AgentIcon icon="code" size={22} />);
    const svg = container.querySelector("svg");
    expect(svg?.getAttribute("width")).toBe("22");
  });
});

describe("AgentBadge", () => {
  it("mostra o nome do agente e a cor do token", () => {
    const { container } = render(
      <AgentBadge mode={{ name: "LeetCode", icon: "code", color: "emerald" }} />,
    );
    const badge = container.querySelector(".agent-badge");
    expect(badge?.textContent).toContain("LeetCode");
    expect(badge?.getAttribute("style")).toContain("--oh-agent-emerald");
  });

  it("cai para a cor padrão quando o agente não tem identidade", () => {
    const { container } = render(<AgentBadge mode={{ name: "Vendas" }} />);
    const badge = container.querySelector(".agent-badge");
    expect(badge?.getAttribute("style")).toContain("--oh-agent-slate");
  });

  it("mostra um rótulo neutro sem agente", () => {
    const { container } = render(<AgentBadge mode={null} fallback="Sugestão" />);
    expect(container.querySelector(".agent-badge")?.textContent).toContain(
      "Sugestão",
    );
  });
});
