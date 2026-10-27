import { fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import type { SuggestionClassification } from "../../core/domain/assistant";
import { LastClassificationStrip } from "./LastClassificationStrip";

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
  block_end_ms: 65_000,
  turn_count: 3,
  classified_at_ms: Date.now() - 30_000,
  abstained: false,
  ...overrides,
});

function renderStrip(
  overrides: Partial<ComponentProps<typeof LastClassificationStrip>> = {},
) {
  const onClassifyNow = vi.fn();
  const onRetry = vi.fn();
  const onOpenSettings = vi.fn();
  const { container } = render(
    <LastClassificationStrip
      mode="classifier"
      classification={null}
      state="listening"
      error={null}
      canClassify
      onClassifyNow={onClassifyNow}
      onRetry={onRetry}
      onOpenSettings={onOpenSettings}
      {...overrides}
    />,
  );
  return { container, onClassifyNow, onRetry, onOpenSettings };
}

describe("LastClassificationStrip", () => {
  it("mostra o estado vazio e permite classificar", () => {
    const { container, onClassifyNow } = renderStrip();
    expect(
      container.querySelector(".classification-strip__title")?.textContent,
    ).toBe("Última classificação");
    expect(
      container.querySelector(".classification-strip__hint")?.textContent,
    ).toContain("Nenhuma classificação ainda");
    fireEvent.click(screen.getByRole("button", { name: "Classificar agora" }));
    expect(onClassifyNow).toHaveBeenCalledOnce();
  });

  it("no modo automático aponta para as configurações", () => {
    const { onOpenSettings } = renderStrip({ mode: "auto" });
    expect(screen.getByText("Modo automático: o classificador está desligado.")).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Ajustar nas Configurações" }));
    expect(onOpenSettings).toHaveBeenCalledOnce();
  });

  it("mostra erro e tenta de novo", () => {
    const { onRetry } = renderStrip({
      error: "timeout",
      classification: null,
    });
    expect(screen.getByText(/Classificador indisponível: timeout/)).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it("mostra cue, decisão, confiança e tempo", () => {
    const { container } = renderStrip({
      classification: classification(),
      state: "ready",
    });
    expect(container.querySelector(".classification-strip__cue")?.textContent).toBe(
      "Pergunta",
    );
    expect(container.querySelector(".classification-strip__decision")?.textContent).toBe(
      "Vai sugerir",
    );
    expect(container.querySelector(".classification-strip__confidence")?.textContent).toBe(
      "90%",
    );
    expect(container.querySelector(".classification-strip__time")?.textContent).toContain(
      "Há 30s",
    );
  });

  it("expande os detalhes com evidência, bloco e backend", () => {
    const { container } = renderStrip({
      classification: classification(),
      state: "ready",
    });
    fireEvent.click(container.querySelector(".classification-strip__summary")!);
    expect(container.querySelector(".classification-strip__details")?.textContent).toContain(
      "pode explicar?",
    );
    expect(container.querySelector(".classification-strip__details")?.textContent).toContain(
      "0:00–1:05 · 3 turnos",
    );
    expect(container.querySelector(".classification-strip__details")?.textContent).toContain(
      "heuristic",
    );
  });

  it("mostra decisão negativa sem abrir sugestão", () => {
    renderStrip({
      classification: classification({
        cue: "no_action",
        should_suggest: false,
        confidence: 0.6,
      }),
      state: "ready",
    });
    expect(screen.getByText("Sem ação")).toBeDefined();
    expect(screen.getByText("Não sugerir")).toBeDefined();
  });
});
