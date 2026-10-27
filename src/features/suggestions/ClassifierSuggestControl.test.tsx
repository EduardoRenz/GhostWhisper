import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import type { SuggestionClassification } from "../../core/domain/assistant";
import { ClassifierSuggestControl } from "./ClassifierSuggestControl";

const classification = (
  overrides: Partial<SuggestionClassification> = {},
): SuggestionClassification => ({
  block_id: "blk_1",
  session_id: "s1",
  cue: "question",
  should_suggest: true,
  confidence: 0.9,
  evidence: "pode explicar?",
  focus: "responder à pergunta",
  backend: "heuristic",
  model: null,
  latency_ms: 0,
  block_start_ms: 0,
  block_end_ms: 1_000,
  turn_count: 2,
  classified_at_ms: 1_000,
  abstained: false,
  ...overrides,
});

function renderControl(
  overrides: Partial<ComponentProps<typeof ClassifierSuggestControl>> = {},
) {
  const onToggle = vi.fn();
  const { container } = render(
    <ClassifierSuggestControl
      paused={false}
      sessionActive
      classification={null}
      state="listening"
      generating={false}
      onToggle={onToggle}
      {...overrides}
    />,
  );
  return { onToggle, container };
}

describe("ClassifierSuggestControl", () => {
  it("mostra o rótulo e o estado de escuta", () => {
    const { container } = renderControl();
    expect(container.querySelector(".classifier-control__label")?.textContent).toBe(
      "Classificador",
    );
    expect(container.querySelector(".classifier-control__state")?.textContent).toBe(
      "Ouvindo bloco…",
    );
  });

  it("mostra pausado e sem escuta", () => {
    const paused = renderControl({ paused: true });
    expect(paused).toBeTruthy();
    expect(screen.getByText("Pausado")).toBeDefined();

    renderControl({ sessionActive: false });
    expect(screen.getByText("Sem escuta")).toBeDefined();
  });

  it("mostra analisando, falha e aguardando", () => {
    renderControl({ state: "analyzing" });
    expect(screen.getByText("Analisando…")).toBeDefined();

    renderControl({ state: "error" });
    expect(screen.getByText("Falha")).toBeDefined();

    renderControl();
    expect(screen.getByText("Ouvindo bloco…")).toBeDefined();
  });

  it("mostra a cue quando vai sugerir", () => {
    renderControl({ classification: classification(), state: "ready" });
    expect(screen.getByText("Vai sugerir: Pergunta")).toBeDefined();
  });

  it("mostra a cue negativa sem repetir o texto", () => {
    renderControl({
      classification: classification({
        cue: "no_action",
        should_suggest: false,
      }),
      state: "ready",
    });
    expect(screen.getByText("Sem ação")).toBeDefined();
  });

  it("chama onToggle no clique", () => {
    const { onToggle } = renderControl();
    fireEvent.click(screen.getByRole("button"));
    expect(onToggle).toHaveBeenCalledOnce();
  });
});
