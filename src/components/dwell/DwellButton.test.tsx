import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DwellButton } from "./DwellButton";
import { DwellProvider } from "./DwellProvider";

function Harness({
  enabled = true,
  active = true,
  onActivate = vi.fn(),
}: {
  enabled?: boolean;
  active?: boolean;
  onActivate?: () => void;
}) {
  return (
    <DwellProvider enabled={enabled} durationMs={500} active={active}>
      <DwellButton onActivate={onActivate}>Resumo</DwellButton>
    </DwellProvider>
  );
}

describe("DwellButton", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("executa a ação ao completar a permanência, sem confirmação", () => {
    const onActivate = vi.fn();
    render(<Harness onActivate={onActivate} />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.pointerOver(button);
    expect(button.getAttribute("data-arming")).toBe("true");

    act(() => vi.advanceTimersByTime(500));

    expect(onActivate).toHaveBeenCalledTimes(1);
    expect(button.getAttribute("data-arming")).toBe("false");
  });

  it("sair antes do tempo cancela", () => {
    const onActivate = vi.fn();
    render(<Harness onActivate={onActivate} />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.pointerOver(button);
    act(() => vi.advanceTimersByTime(200));
    fireEvent.pointerOut(button);
    act(() => vi.advanceTimersByTime(500));

    expect(onActivate).not.toHaveBeenCalled();
    expect(button.getAttribute("data-arming")).toBe("false");
  });

  it("clique executa na hora", () => {
    const onActivate = vi.fn();
    render(<Harness onActivate={onActivate} />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.click(button);

    expect(onActivate).toHaveBeenCalledTimes(1);
  });

  it("mover dentro do botão não reinicia nem duplica", () => {
    const onActivate = vi.fn();
    render(<Harness onActivate={onActivate} />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.pointerOver(button);
    act(() => vi.advanceTimersByTime(250));
    fireEvent.pointerMove(button);
    act(() => vi.advanceTimersByTime(250));

    expect(onActivate).toHaveBeenCalledTimes(1);
  });

  it("não repete a ação enquanto o cursor permanece após completar", () => {
    const onActivate = vi.fn();
    render(<Harness onActivate={onActivate} />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.pointerOver(button);
    act(() => vi.advanceTimersByTime(500));
    expect(onActivate).toHaveBeenCalledTimes(1);

    fireEvent.pointerMove(button);
    fireEvent.pointerOver(button);
    act(() => vi.advanceTimersByTime(500));
    expect(onActivate).toHaveBeenCalledTimes(1);
  });

  it("desligado ou sem sessão não arma no hover", () => {
    const off = vi.fn();
    const { rerender } = render(
      <Harness enabled={false} onActivate={off} />,
    );
    const button = screen.getByRole("button", { name: "Resumo" });
    fireEvent.pointerOver(button);
    act(() => vi.advanceTimersByTime(500));
    expect(off).not.toHaveBeenCalled();
    expect(button.getAttribute("data-arming")).toBe("false");

    rerender(<Harness active={false} onActivate={off} />);
    fireEvent.pointerOver(button);
    act(() => vi.advanceTimersByTime(500));
    expect(off).not.toHaveBeenCalled();
  });
});
