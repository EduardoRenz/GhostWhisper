import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DwellButton } from "./DwellButton";
import { DwellProvider } from "./DwellProvider";

const show = vi.hoisted(() => vi.fn(async () => undefined));
const hide = vi.hoisted(() => vi.fn(async () => undefined));
const triggerHover = vi.hoisted(() => vi.fn(async () => undefined));

vi.mock("../../adapters/ipc/dwell", () => ({
  dwellShow: show,
  dwellHide: hide,
  dwellResult: vi.fn(async () => undefined),
  dwellContext: vi.fn(async () => 500),
  dwellTriggerHover: triggerHover,
}));

function Harness({
  enabled = true,
  onActivate = vi.fn(),
}: {
  enabled?: boolean;
  onActivate?: () => void;
}) {
  return (
    <DwellProvider enabled={enabled} durationMs={500} active>
      <DwellButton triggerId="test-trigger" onActivate={onActivate}>
        Resumo
      </DwellButton>
    </DwellProvider>
  );
}

describe("DwellButton", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    show.mockClear();
    hide.mockClear();
    triggerHover.mockClear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("abre a confirmação após a permanência", () => {
    render(<Harness />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.pointerOver(button);
    expect(button.getAttribute("data-arming")).toBe("true");

    act(() => vi.advanceTimersByTime(500));

    expect(show).toHaveBeenCalledTimes(1);
    expect(show).toHaveBeenCalledWith(
      { x: 0, y: 0, width: 0, height: 0 },
      500,
    );
    expect(button.getAttribute("data-arming")).toBe("true");
  });

  it("sair antes do tempo não abre", () => {
    render(<Harness />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.pointerOver(button);
    act(() => vi.advanceTimersByTime(200));
    fireEvent.pointerOut(button);
    act(() => vi.advanceTimersByTime(500));

    expect(show).not.toHaveBeenCalled();
    expect(button.getAttribute("data-arming")).toBe("false");
  });

  it("clique executa na hora, sem confirmação", () => {
    const onActivate = vi.fn();
    render(<Harness onActivate={onActivate} />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.click(button);

    expect(onActivate).toHaveBeenCalledTimes(1);
    expect(show).not.toHaveBeenCalled();
  });

  it("desligado não arma no hover", () => {
    render(<Harness enabled={false} />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.pointerOver(button);
    act(() => vi.advanceTimersByTime(500));

    expect(show).not.toHaveBeenCalled();
    expect(button.getAttribute("data-arming")).toBe("false");
  });

  it("avisa a confirmação quando o cursor sai do gatilho", () => {
    render(<Harness />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.pointerOver(button);
    act(() => vi.advanceTimersByTime(500));
    fireEvent.pointerOut(button);

    expect(triggerHover).toHaveBeenLastCalledWith(false);
  });

  it("avisa a confirmação quando o cursor volta ao gatilho", () => {
    render(<Harness />);
    const button = screen.getByRole("button", { name: "Resumo" });

    fireEvent.pointerOver(button);
    act(() => vi.advanceTimersByTime(500));
    fireEvent.pointerOut(button);
    fireEvent.pointerOver(button);

    expect(triggerHover).toHaveBeenLastCalledWith(true);
  });
});
