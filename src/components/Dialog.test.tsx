import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Dialog } from "./Dialog";

describe("Dialog", () => {
  it("não renderiza nada quando fechado", () => {
    const { container } = render(
      <Dialog open={false} title="Teste" onClose={() => {}}>
        conteúdo
      </Dialog>,
    );
    expect(container.querySelector('[role="dialog"]')).toBeNull();
  });

  it("renderiza o conteúdo com o contrato de acessibilidade", () => {
    render(
      <Dialog
        open
        title="Editar agente"
        subtitle="Ajuste as propriedades"
        onClose={() => {}}
        footer={<button type="button">Salvar</button>}
      >
        <label>
          Nome
          <input aria-label="Nome" />
        </label>
      </Dialog>,
    );

    const dialog = screen.getByRole("dialog", { name: "Editar agente" });
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(screen.getByText("Ajuste as propriedades")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Salvar" })).toBeTruthy();
  });

  it("fecha pelo botão de fechar", () => {
    const onClose = vi.fn();
    render(
      <Dialog open title="Teste" onClose={onClose}>
        conteúdo
      </Dialog>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Fechar" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("fecha pelo backdrop", () => {
    const onClose = vi.fn();
    render(
      <Dialog open title="Teste" onClose={onClose}>
        conteúdo
      </Dialog>,
    );
    // O modal é renderizado em portal no `body`.
    const backdrop = document.querySelector(".dialog__backdrop");
    expect(backdrop).not.toBeNull();
    fireEvent.click(backdrop as Element);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("fecha com Esc e não deixa o evento chegar na janela", () => {
    const onClose = vi.fn();
    const windowKey = vi.fn();
    window.addEventListener("keydown", windowKey);
    render(
      <Dialog open title="Teste" onClose={onClose}>
        <input aria-label="Nome" />
      </Dialog>,
    );

    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(windowKey).not.toHaveBeenCalled();
    window.removeEventListener("keydown", windowKey);
  });

  it("prende o Tab dentro do modal", () => {
    render(
      <Dialog
        open
        title="Teste"
        onClose={() => {}}
        footer={<button type="button">Salvar</button>}
      >
        <input aria-label="Nome" />
      </Dialog>,
    );

    const fechar = screen.getByRole("button", { name: "Fechar" });
    const salvar = screen.getByRole("button", { name: "Salvar" });
    const nome = screen.getByLabelText("Nome");

    // O foco começa no primeiro campo do formulário.
    expect(document.activeElement).toBe(nome);

    // Último → primeiro.
    salvar.focus();
    fireEvent.keyDown(salvar, { key: "Tab" });
    expect(document.activeElement).toBe(fechar);

    // Primeiro → último (Shift+Tab).
    fechar.focus();
    fireEvent.keyDown(fechar, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(salvar);
  });
});
