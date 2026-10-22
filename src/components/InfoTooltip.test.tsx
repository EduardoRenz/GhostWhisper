import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { InfoTooltip } from "./InfoTooltip";

/** O balão é renderizado em portal no `body`. */
function bubbleOf(button: HTMLElement): HTMLElement {
  const id = button.getAttribute("aria-describedby");
  const bubble = id ? document.getElementById(id) : null;
  if (!bubble) {
    throw new Error("balão do tooltip não encontrado");
  }
  return bubble;
}

describe("InfoTooltip", () => {
  it("liga o botão ao texto por aria-describedby", () => {
    render(
      <InfoTooltip label="Prompt de Sugestão" text="Explicação do campo." />,
    );

    const button = screen.getByRole("button", {
      name: "Ajuda: Prompt de Sugestão",
    });
    const bubble = bubbleOf(button);

    expect(bubble.getAttribute("role")).toBe("tooltip");
    expect(bubble.textContent).toBe("Explicação do campo.");
    // Em portal no body: não sofre com o overflow dos contêineres.
    expect(bubble.parentElement).toBe(document.body);
  });

  it("abre no foco e fecha no blur", async () => {
    render(<InfoTooltip label="Campo" text="Ajuda." />);
    const button = screen.getByRole("button", { name: "Ajuda: Campo" });
    const bubble = bubbleOf(button);

    expect(bubble.getAttribute("data-open")).toBe("false");
    fireEvent.focus(button);
    await waitFor(() =>
      expect(bubble.getAttribute("data-open")).toBe("true"),
    );

    fireEvent.blur(button);
    await waitFor(() =>
      expect(bubble.getAttribute("data-open")).toBe("false"),
    );
  });

  it("abre no hover e fecha ao sair", async () => {
    render(<InfoTooltip label="Campo" text="Ajuda." />);
    const button = screen.getByRole("button", { name: "Ajuda: Campo" });
    const bubble = bubbleOf(button);

    fireEvent.pointerEnter(button);
    await waitFor(() =>
      expect(bubble.getAttribute("data-open")).toBe("true"),
    );

    fireEvent.pointerLeave(button);
    await waitFor(() =>
      expect(bubble.getAttribute("data-open")).toBe("false"),
    );
  });

  it("é um botão de ajuda (não dispara submit do formulário)", () => {
    const { container } = render(
      <form onSubmit={() => {}}>
        <InfoTooltip label="Campo" text="Ajuda." />
      </form>,
    );
    const button = container.querySelector("button");
    expect(button?.getAttribute("type")).toBe("button");
    fireEvent.click(button as HTMLButtonElement);
  });
});
