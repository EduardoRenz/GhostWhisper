import { describe, expect, it } from "vitest";
import { isBuiltinMode, summaryTopic, type MeetingSummary } from "./assistant";

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
    expect(isBuiltinMode("meeting")).toBe(true);
    expect(isBuiltinMode("leetcode")).toBe(true);
    expect(isBuiltinMode("custom-vendas")).toBe(false);
  });
});
