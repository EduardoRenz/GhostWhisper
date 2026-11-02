import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AskBar } from "./AskBar";

const base = {
  value: "",
  onChange: vi.fn(),
  onSubmit: vi.fn(),
  onCapture: vi.fn(),
  focusSignal: 0,
  busy: false,
};

describe("AskBar (barra Perguntar)", () => {
  it("mostra a lupa, o campo e o botão Capturar Tela", () => {
    render(<AskBar {...base} />);
    expect(screen.getByLabelText("Perguntar ao GhostWhisper")).toBeDefined();
    expect(screen.getByLabelText("Capturar Tela")).toBeDefined();
  });

  it("mostra os atalhos reais (Ctrl K e Alt C)", () => {
    render(<AskBar {...base} />);
    const keys = [...document.querySelectorAll(".shortcut-badge")].map(
      (key) => key.textContent,
    );
    expect(keys).toEqual(["Ctrl", "K", "Alt", "C"]);
  });

  it("envia com Enter", () => {
    const onSubmit = vi.fn();
    render(<AskBar {...base} value="o que eu disse?" onSubmit={onSubmit} />);

    fireEvent.keyDown(screen.getByLabelText("Perguntar ao GhostWhisper"), {
      key: "Enter",
    });

    expect(onSubmit).toHaveBeenCalledWith("o que eu disse?");
  });

  it("Enter com campo vazio também chama (a captura decide o prompt)", () => {
    const onSubmit = vi.fn();
    render(<AskBar {...base} value="   " onSubmit={onSubmit} />);

    fireEvent.keyDown(screen.getByLabelText("Perguntar ao GhostWhisper"), {
      key: "Enter",
    });

    expect(onSubmit).toHaveBeenCalledWith("");
  });

  it("Capturar Tela dispara a captura", () => {
    const onCapture = vi.fn();
    render(<AskBar {...base} onCapture={onCapture} />);

    fireEvent.click(screen.getByLabelText("Capturar Tela"));

    expect(onCapture).toHaveBeenCalledTimes(1);
  });

  it("mostra as perguntas prontas em repouso e envia ao clicar", () => {
    const onPreset = vi.fn();
    render(
      <AskBar
        {...base}
        presets={["Listar as ações", "Resumir a reunião"]}
        onPreset={onPreset}
      />,
    );

    fireEvent.click(screen.getByText("Listar as ações"));

    expect(onPreset).toHaveBeenCalledWith("Listar as ações");
  });

  it("esconde as perguntas prontas ao digitar", () => {
    render(
      <AskBar
        {...base}
        value="digitando"
        presets={["Listar as ações"]}
        onPreset={vi.fn()}
      />,
    );

    expect(screen.queryByText("Listar as ações")).toBeNull();
  });

  it("desabilita o campo e a captura enquanto ocupa", () => {
    render(<AskBar {...base} busy />);
    expect(
      screen.getByLabelText("Perguntar ao GhostWhisper").hasAttribute("disabled"),
    ).toBe(true);
    expect(
      screen.getByLabelText("Capturar Tela").hasAttribute("disabled"),
    ).toBe(true);
  });

  // --- Pill de contexto como checkbox (BR-106) ---

  it("sem contexto configurado não mostra a pill", () => {
    render(<AskBar {...base} contextState="hidden" />);
    expect(screen.queryByTestId("context-pill")).toBeNull();
  });

  it("passa o checkbox do contexto: a pill inteira vira checkbox", () => {
    const onToggleContext = vi.fn();
    render(
      <AskBar
        {...base}
        contextState="active"
        contextEvent={{
          available: true,
          app: "google-chrome",
          window_title: "Doc",
          preview: "texto",
          captured_at_ms: 1,
        }}
        contextChecked
        onToggleContext={onToggleContext}
      />,
    );
    const pill = screen.getByTestId("context-pill");
    expect(pill.getAttribute("role")).toBe("checkbox");
    expect(pill.getAttribute("aria-checked")).toBe("true");

    fireEvent.click(pill);
    expect(onToggleContext).toHaveBeenCalledTimes(1);
  });

  it("a opção de contexto fica visível mesmo sem contexto agora", () => {
    // Pedido do usuário: a barra nunca pode esconder a opção de incluir/
    // excluir — estado "sem contexto" também mostra o checkbox.
    render(
      <AskBar
        {...base}
        contextState="empty"
        contextEvent={null}
        contextChecked={false}
        onToggleContext={vi.fn()}
      />,
    );
    const pill = screen.getByTestId("context-pill");
    expect(pill.getAttribute("role")).toBe("checkbox");
    expect(pill.getAttribute("aria-checked")).toBe("false");
    expect(pill.textContent).toContain("Sem contexto de tela");
  });
});
