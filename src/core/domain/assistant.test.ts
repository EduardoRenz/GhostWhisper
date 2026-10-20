import { describe, expect, it } from "vitest";
import {
  AGENT_COLOR_TOKENS,
  AGENT_ICON_IDS,
  agentColorToken,
  agentIconId,
  isAgentColorToken,
  isAgentIconId,
  isBuiltinMode,
  summaryTopic,
  type MeetingSummary,
} from "./assistant";

const summary = (
  subject: string | null,
  markdown: string,
): MeetingSummary => ({
  session_id: "s1",
  subject,
  markdown,
  summarized_from_ms: 0,
  summarized_until_ms: 10_000,
  updated_at_ms: 1_700_000_000_000,
});

describe("summaryTopic", () => {
  it("prefere o subject gerado pelo modelo", () => {
    expect(summaryTopic(summary("Planejamento", "## Visão geral"))).toBe(
      "Planejamento",
    );
  });

  it("cai para o primeiro heading quando não há subject", () => {
    expect(summaryTopic(summary(null, "## Decisões confirmadas\n\nok"))).toBe(
      "Decisões confirmadas",
    );
  });

  it("usa um rótulo neutro sem subject nem heading", () => {
    expect(summaryTopic(summary("  ", "só texto"))).toBe("Resumo sem título");
    expect(summaryTopic(null)).toBe("Resumo sem título");
  });
});

describe("isBuiltinMode", () => {
  it("distingue embutidos de personalizados", () => {
    expect(isBuiltinMode("default")).toBe(true);
    expect(isBuiltinMode("meeting")).toBe(true);
    expect(isBuiltinMode("leetcode")).toBe(true);
    expect(isBuiltinMode("custom-vendas")).toBe(false);
  });
});

describe("identidade do agente (BR-037)", () => {
  it("aceita todo o catálogo de ícones e cores", () => {
    for (const icon of AGENT_ICON_IDS) {
      expect(isAgentIconId(icon)).toBe(true);
    }
    for (const color of AGENT_COLOR_TOKENS) {
      expect(isAgentColorToken(color)).toBe(true);
    }
    expect(isAgentIconId("rocket")).toBe(false);
    expect(isAgentColorToken("#00f2fe")).toBe(false);
  });

  it("usa o ícone e a cor do agente", () => {
    expect(agentIconId({ icon: "code" })).toBe("code");
    expect(agentColorToken({ color: "emerald" })).toBe("emerald");
  });

  it("cai para o padrão quando o campo falta ou é desconhecido", () => {
    // Settings antigas não têm `icon`/`color` (BR-037).
    expect(agentIconId({})).toBe("sparkle");
    expect(agentColorToken({})).toBe("slate");
    expect(agentIconId({ icon: "rocket" })).toBe("sparkle");
    expect(agentColorToken({ color: "chartreuse" })).toBe("slate");
    expect(agentIconId(null)).toBe("sparkle");
    expect(agentColorToken(undefined)).toBe("slate");
  });
});
