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

  it("desabilita o campo e a captura enquanto ocupa", () => {
    render(<AskBar {...base} busy />);
    expect(
      screen.getByLabelText("Perguntar ao GhostWhisper").hasAttribute("disabled"),
    ).toBe(true);
    expect(
      screen.getByLabelText("Capturar Tela").hasAttribute("disabled"),
    ).toBe(true);
  });
});
