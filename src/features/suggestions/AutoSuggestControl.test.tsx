import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AutoSuggestControl } from "./AutoSuggestControl";

const cadence = (overrides: Partial<{
  enabled: boolean;
  running: boolean;
  remaining_ms: number;
  awaiting_speech: boolean;
  window_ms: number;
  generating: boolean;
}> = {}) => ({
  enabled: true,
  running: true,
  remaining_ms: 12_000,
  awaiting_speech: false,
  window_ms: 20_000,
  generating: false,
  ...overrides,
});

describe("AutoSuggestControl", () => {
  it("mostra o texto, a contagem e o anel de progresso", () => {
    const { container } = render(
      <AutoSuggestControl cadence={cadence()} paused={false} onToggle={() => {}} />,
    );
    // O rótulo do controle é sempre visível.
    expect(container.querySelector(".auto-control__label")?.textContent).toBe(
      "Auto-sugerir",
    );
    // A contagem regressiva fica na pill.
    expect(container.querySelector(".auto-control__count")?.textContent).toBe(
      "12s",
    );
    // E o anel existe, com progresso.
    expect(container.querySelector(".auto-control__progress")).not.toBeNull();
    const offset = Number(
      container.querySelector(".auto-control__progress")?.getAttribute("stroke-dashoffset"),
    );
    expect(offset).toBeGreaterThan(0);
    expect(offset).toBeLessThan(60);
  });

  it("mostra “Pausado” quando o automático está desligado", () => {
    const { container } = render(
      <AutoSuggestControl
        cadence={cadence({ enabled: false })}
        paused
        onToggle={() => {}}
      />,
    );
    expect(container.querySelector(".auto-control__count")?.textContent).toBe(
      "Pausado",
    );
    expect(container.querySelector(".auto-control--paused")).not.toBeNull();
    // O texto do controle continua lá.
    expect(container.querySelector(".auto-control__label")?.textContent).toBe(
      "Auto-sugerir",
    );
  });

  it("mostra “Aguardando” quando a janela venceu sem fala nova", () => {
    const { container } = render(
      <AutoSuggestControl
        cadence={cadence({ remaining_ms: 0, awaiting_speech: true })}
        paused={false}
        onToggle={() => {}}
      />,
    );
    expect(container.querySelector(".auto-control__count")?.textContent).toBe(
      "Aguardando",
    );
    expect(container.querySelector(".auto-control--waiting")).not.toBeNull();
  });

  it("mostra “Sem escuta” quando não há sessão", () => {
    const { container } = render(
      <AutoSuggestControl
        cadence={cadence({ running: false, remaining_ms: 0 })}
        paused={false}
        onToggle={() => {}}
      />,
    );
    expect(container.querySelector(".auto-control__count")?.textContent).toBe(
      "Sem escuta",
    );
    expect(container.querySelector(".auto-control")?.getAttribute("title")).toContain(
      "Sem sessão de escuta",
    );
  });

  it("mostra “Gerando” e o arco indeterminado durante a geração", () => {
    const { container } = render(
      <AutoSuggestControl
        cadence={cadence({ remaining_ms: 0, generating: true })}
        paused={false}
        onToggle={() => {}}
      />,
    );
    expect(container.querySelector(".auto-control__count")?.textContent).toBe(
      "Gerando",
    );
    expect(container.querySelector(".auto-control--generating")).not.toBeNull();
    expect(
      container
        .querySelector(".auto-control__progress")
        ?.getAttribute("stroke-dasharray"),
    ).toContain(" ");
  });

  it("esvazia o anel conforme o tempo restante", () => {
    const { container } = render(
      <AutoSuggestControl cadence={cadence()} paused={false} onToggle={() => {}} />,
    );
    const progress = container.querySelector(".auto-control__progress");
    const offset = Number(progress?.getAttribute("stroke-dashoffset"));
    // 12 s restantes de um ciclo de 20 s: o anel está 60% cheio.
    expect(offset).toBeGreaterThan(0);
    expect(offset).toBeLessThan(60);
  });

  it("liga e desliga o automático ao clicar", () => {
    const onToggle = vi.fn();
    render(
      <AutoSuggestControl cadence={cadence()} paused={false} onToggle={onToggle} />,
    );
    fireEvent.click(screen.getByRole("button"));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });
});
