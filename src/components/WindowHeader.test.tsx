import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { WindowHeader } from "./WindowHeader";

function renderHeader() {
  const onStartDrag = vi.fn();
  const onClose = vi.fn();
  const { container } = render(
    <WindowHeader
      title="Transcrição"
      subtitle="Ao vivo"
      actions={
        <button type="button" aria-label="Auto-sugerir">
          Auto
        </button>
      }
      onStartDrag={onStartDrag}
      onClose={onClose}
    />,
  );

  return {
    onStartDrag,
    onClose,
    container,
    header: container.querySelector(".window-header") as HTMLElement,
  };
}

/** O gesto parte do mousedown, que chega depois da ativação (ISS-063). */
function dragFrom(target: Element) {
  fireEvent.mouseDown(target, { button: 0 });
}

describe("WindowHeader", () => {
  it("arrasta pela área vazia do header", () => {
    const { header, onStartDrag } = renderHeader();

    dragFrom(header);

    expect(onStartDrag).toHaveBeenCalledTimes(1);
  });

  it("arrasta pelo título e pelo subtítulo", () => {
    const { onStartDrag } = renderHeader();

    dragFrom(screen.getByText("Transcrição"));
    dragFrom(screen.getByText("Ao vivo"));

    expect(onStartDrag).toHaveBeenCalledTimes(2);
  });

  it("a alça continua arrastando, sem disparar o header em dobro", () => {
    const { onStartDrag } = renderHeader();

    dragFrom(screen.getByLabelText("Mover janela"));

    expect(onStartDrag).toHaveBeenCalledTimes(1);
  });

  it("não arrasta ao pressionar controle de ação", () => {
    const { onStartDrag } = renderHeader();

    dragFrom(screen.getByRole("button", { name: "Auto-sugerir" }));

    expect(onStartDrag).not.toHaveBeenCalled();
  });

  it("não arrasta ao clicar em fechar", () => {
    const { onStartDrag, onClose } = renderHeader();
    const close = screen.getByRole("button", { name: "Fechar" });

    dragFrom(close);
    fireEvent.click(close);

    expect(onStartDrag).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("só o botão primário arrasta", () => {
    const { header, onStartDrag } = renderHeader();

    fireEvent.mouseDown(header, { button: 2 });

    expect(onStartDrag).not.toHaveBeenCalled();
  });

  it("respeita áreas marcadas para não arrastar", () => {
    const { container, onStartDrag } = renderHeader();
    const optOut = document.createElement("span");
    optOut.dataset.noWindowDrag = "";
    optOut.textContent = "Conteúdo";
    container.querySelector(".window-header")!.append(optOut);

    dragFrom(optOut);

    expect(onStartDrag).not.toHaveBeenCalled();
  });
});
