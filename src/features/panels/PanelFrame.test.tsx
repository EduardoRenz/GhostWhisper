import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const startWindowResize = vi.fn().mockResolvedValue(undefined);
const panelHide = vi.fn();
const startWindowDrag = vi.fn();

vi.mock("../../adapters/ipc/panels", () => ({
  panelHide: (...args: unknown[]) => panelHide(...args),
}));

vi.mock("../../adapters/ipc/windowDrag", () => ({
  startWindowDrag: (...args: unknown[]) => startWindowDrag(...args),
  startWindowResize: (...args: unknown[]) => startWindowResize(...args),
}));

import { PanelFrame } from "./PanelFrame";

/** O gesto parte do mousedown, que chega depois da ativação (ISS-063). */
function dragFrom(element: Element) {
  fireEvent.mouseDown(element, { button: 0 });
}

describe("PanelFrame", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renderiza o painel e as 8 alças de resize", () => {
    const { container } = render(
      <PanelFrame label="chat" title="Resumo">
        conteúdo
      </PanelFrame>,
    );

    expect(container.querySelector(".panel")).not.toBeNull();
    expect(container.querySelectorAll(".panel__resize")).toHaveLength(8);
    expect(screen.getByText("conteúdo")).toBeDefined();
  });

  it("inicia o resize na direção da alça pressionada", () => {
    const { container } = render(
      <PanelFrame label="chat" title="Resumo">
        x
      </PanelFrame>,
    );

    fireEvent.mouseDown(
      container.querySelector('[data-direction="SouthEast"]')!,
      { button: 0 },
    );

    expect(startWindowResize).toHaveBeenCalledWith("SouthEast");
  });

  it("esconde a janela ao clicar em Fechar", () => {
    render(
      <PanelFrame label="suggestions" title="Sugestões">
        x
      </PanelFrame>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Fechar" }));
    expect(panelHide).toHaveBeenCalledWith("suggestions");
  });

  it("usa o header padrão: grip à esquerda e fechar à direita", () => {
    const { container } = render(
      <PanelFrame label="chat" title="Transcrição ao Vivo">
        x
      </PanelFrame>,
    );
    const header = container.querySelector(".window-header")!;
    const drag = screen.getByLabelText("Mover janela");
    const close = screen.getByRole("button", { name: "Fechar" });

    expect(header.firstElementChild).toBe(drag);
    expect(header.lastElementChild).toBe(close);

    fireEvent.mouseDown(drag);
    expect(startWindowDrag).toHaveBeenCalledTimes(1);
  });

  it("arrasta pelo header inteiro, não só pelo grip", () => {
    const { container } = render(
      <PanelFrame label="chat" title="Transcrição ao Vivo">
        x
      </PanelFrame>,
    );

    dragFrom(container.querySelector(".window-header__title")!);

    expect(startWindowDrag).toHaveBeenCalledTimes(1);
  });

  it("não arrasta ao clicar em um controle do header", () => {
    const { container } = render(
      <PanelFrame
        label="suggestions"
        title="Sugestões"
        actions={
          <button type="button" aria-label="Auto-sugerir">
            Auto
          </button>
        }
      >
        x
      </PanelFrame>,
    );

    dragFrom(
      container.querySelector(
        '.window-header__actions button[aria-label="Auto-sugerir"]',
      )!,
    );

    expect(startWindowDrag).not.toHaveBeenCalled();
  });
});
