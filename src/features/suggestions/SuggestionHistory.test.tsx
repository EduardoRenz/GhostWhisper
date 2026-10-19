import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Suggestion } from "../../core/domain/assistant";
import { SuggestionHistory } from "./SuggestionHistory";

const item = (overrides: Partial<Suggestion> = {}): Suggestion => ({
  session_id: "s1",
  mode_id: "system_design",
  mode_name: "System Design",
  mode_icon: "layers",
  mode_color: "coral",
  phase: "full",
  title: "",
  text: "## Planejar um System Design de Fintech",
  at_ms: 1_000,
  context_until_ms: 1_000,
  message_id: "m1",
  skipped: false,
  ...overrides,
});

describe("SuggestionHistory", () => {
  beforeEach(() => {
    // jsdom não implementa `scrollIntoView`.
    Element.prototype.scrollIntoView = vi.fn();
  });

  it("mostra a seção com o contador de anteriores", () => {
    const { container } = render(
      <SuggestionHistory
        items={[item(), item({ at_ms: 2_000, message_id: "m2" })]}
        pinnedId={null}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(container.querySelector(".history__title")?.textContent).toBe(
      "Histórico de Sugestões da Sessão",
    );
    expect(container.querySelector(".history__count")?.textContent).toBe(
      "2 anteriores",
    );
  });

  it("colapsa e expande a seção", () => {
    const { container } = render(
      <SuggestionHistory
        items={[item()]}
        pinnedId={null}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    const head = container.querySelector(".history__head");
    expect(head?.getAttribute("aria-expanded")).toBe("true");
    fireEvent.click(head as HTMLElement);
    expect(head?.getAttribute("aria-expanded")).toBe("false");
    expect(container.querySelector(".history__list")).toBeNull();
  });

  it("cada item mostra o ícone do agente, o título e a pill do agente", () => {
    const { container } = render(
      <SuggestionHistory
        items={[item()]}
        pinnedId={null}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(container.querySelector(".history__item-title")?.textContent).toBe(
      "Planejar um System Design de Fintech",
    );
    const badge = container.querySelector(".agent-badge");
    expect(badge?.textContent).toContain("System Design");
    expect(badge?.getAttribute("style")).toContain("--oh-agent-coral");
  });

  it("usa o título gerado pelo agente (Subject) quando existe", () => {
    const { container } = render(
      <SuggestionHistory
        items={[
          item({
            title: "Como inverter uma árvore binária em Python",
            text: "## Easy\n\nEu inverteria com recursão.",
          }),
        ]}
        pinnedId={null}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(container.querySelector(".history__item-title")?.textContent).toBe(
      "Como inverter uma árvore binária em Python",
    );
  });

  it("expande o item no lugar, sem trocar a suggestion do topo", () => {
    const { container } = render(
      <SuggestionHistory
        items={[item()]}
        pinnedId={null}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    const head = container.querySelector(".history__item-head");
    fireEvent.click(head as HTMLElement);
    expect(container.querySelector(".history__item-body")).not.toBeNull();
    expect(head?.getAttribute("aria-expanded")).toBe("true");
    // O scroll vai até o bloco expandido (CA-04.2).
    expect(Element.prototype.scrollIntoView).toHaveBeenCalled();
  });

  it("copia e fixa o item expandido", () => {
    const onCopy = vi.fn();
    const onPin = vi.fn();
    render(
      <SuggestionHistory
        items={[item()]}
        pinnedId={null}
        onCopy={onCopy}
        onPin={onPin}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Planear|Planejar/ }));
    fireEvent.click(screen.getByRole("button", { name: "Copiar" }));
    fireEvent.click(screen.getByRole("button", { name: "Fixar na conversa" }));
    expect(onCopy).toHaveBeenCalledTimes(1);
    expect(onPin).toHaveBeenCalledTimes(1);
  });

  it("não mostra nada sem itens", () => {
    const { container } = render(
      <SuggestionHistory
        items={[]}
        pinnedId={null}
        onCopy={() => {}}
        onPin={() => {}}
      />,
    );
    expect(container.querySelector(".history")).toBeNull();
  });
});
