import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Suggestion } from "../../core/domain/assistant";
import { SuggestionCard } from "./SuggestionCard";

const suggestion = (overrides: Partial<Suggestion> = {}): Suggestion => ({
  session_id: "s1",
  mode_id: "leetcode",
  mode_name: "LeetCode",
  mode_icon: "code",
  mode_color: "emerald",
  generation_id: "g1",
  trigger: "auto",
  source: null,
  phase: "full",
  title: "",
  text: "## Estratégia Sliding Window\n\nUse dois ponteiros.",
  at_ms: 1_000,
  context_until_ms: 1_000,
  message_id: null,
  skipped: false,
  ...overrides,
});

describe("SuggestionCard", () => {
  it("mostra a tag, o título derivativo e a pill do agente", () => {
    const { container } = render(
      <SuggestionCard
        current={suggestion()}
        quick={null}
        pinned={false}
        copied={false}
        awaitingFull={false}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(container.querySelector(".suggestion-card__tag")?.textContent).toContain(
      "Sugestão Atual",
    );
    expect(container.querySelector(".suggestion-card__title")?.textContent).toBe(
      "Estratégia Sliding Window",
    );
    // A pill do agente carrega o nome e a cor dele.
    const badge = container.querySelector(".agent-badge");
    expect(badge?.textContent).toContain("LeetCode");
    expect(badge?.getAttribute("style")).toContain("--oh-agent-emerald");
  });

  it("mostra o modelo que respondeu (BR-051)", () => {
    const { container } = render(
      <SuggestionCard
        current={suggestion()}
        quick={null}
        pinned={false}
        copied={false}
        awaitingFull={false}
        llmSource={{
          source_id: "fonte-1",
          label: "DeepSeek",
          provider: "deepseek",
          model: "deepseek-reasoner",
          scope: "agent:leetcode",
          is_fallback: false,
          latency_ms: 142,
        }}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    const badge = container.querySelector(".llm-source-badge");
    expect(badge?.textContent).toContain("deepseek-reasoner");
    // O tooltip traz a fonte completa.
    expect(badge?.getAttribute("title")).toBe("DeepSeek · deepseek-reasoner");
  });

  it("marca o modelo que respondeu em fallback", () => {
    const { container } = render(
      <SuggestionCard
        current={suggestion()}
        quick={null}
        pinned={false}
        copied={false}
        awaitingFull={false}
        llmSource={{
          source_id: "fonte-2",
          label: "OpenAI",
          provider: "openai",
          model: "gpt-4o-mini",
          scope: "agent:leetcode",
          is_fallback: true,
          latency_ms: 90,
        }}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    const badge = container.querySelector(".llm-source-badge");
    expect(badge?.className).toContain("llm-source-badge--fallback");
    expect(badge?.getAttribute("title")).toContain("fallback");
  });

  it("sem uso de IA, não mostra o pill de modelo", () => {
    const { container } = render(
      <SuggestionCard
        current={suggestion()}
        quick={null}
        pinned={false}
        copied={false}
        awaitingFull={false}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(container.querySelector(".llm-source-badge")).toBeNull();
  });

  it("usa o título gerado pelo agente (Subject) quando existe", () => {
    const { container } = render(
      <SuggestionCard
        current={suggestion({
          title: "Como inverter uma árvore binária em Python",
          text: "## Easy\n\nEu inverteria com recursão.",
        })}
        quick={null}
        pinned={false}
        copied={false}
        awaitingFull={false}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(container.querySelector(".suggestion-card__title")?.textContent).toBe(
      "Como inverter uma árvore binária em Python",
    );
  });

  it("mostra a resposta rápida e a completa em blocos próprios (BR-038)", () => {
    const { container } = render(
      <SuggestionCard
        current={suggestion()}
        quick={suggestion({ text: "Proponho sliding window", phase: "quick" })}
        pinned={false}
        copied={false}
        awaitingFull={false}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(
      container.querySelector(".suggestion-block--quick")?.textContent,
    ).toContain("Proponho sliding window");
    expect(
      container.querySelector(".suggestion-block--full")?.textContent,
    ).toContain("Use dois ponteiros");
    // O chip de metadados da resposta completa.
    expect(container.querySelector(".suggestion-chip")?.textContent).toBe(
      "Markdown · Explicativo",
    );
  });

  it("copia e fixa pelo cartão", () => {
    const onCopy = vi.fn();
    const onPin = vi.fn();
    render(
      <SuggestionCard
        current={suggestion()}
        quick={null}
        pinned={false}
        copied={false}
        awaitingFull={false}
        onCopy={onCopy}
        onPin={onPin}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Copiar" }));
    fireEvent.click(screen.getByRole("button", { name: "Fixar na conversa" }));
    expect(onCopy).toHaveBeenCalledTimes(1);
    expect(onPin).toHaveBeenCalledTimes(1);
  });

  it("mostra “Gerando resposta completa…” enquanto só a rápida existe", () => {
    const { container } = render(
      <SuggestionCard
        current={null}
        quick={suggestion({ text: "resposta curta", phase: "quick" })}
        pinned={false}
        copied={false}
        awaitingFull
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(
      container.querySelector(".suggestion-block__pending")?.textContent,
    ).toContain("Gerando resposta completa");
  });

  it("esconde o feedback quando a completa chega", () => {
    const { container } = render(
      <SuggestionCard
        current={suggestion()}
        quick={suggestion({ text: "resposta curta", phase: "quick" })}
        pinned={false}
        copied={false}
        awaitingFull
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(container.querySelector(".suggestion-block__pending")).toBeNull();
  });

  it("não mostra nada sem sugestão", () => {
    const { container } = render(
      <SuggestionCard
        current={null}
        quick={null}
        pinned={false}
        copied={false}
        awaitingFull={false}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(container.querySelector(".suggestion-card")).toBeNull();
  });
});
