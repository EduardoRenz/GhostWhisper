import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SessionPill } from "./SessionPill";

const NOW = new Date("2026-10-18T12:00:00Z").getTime();

const base = {
  confirming: false,
  busy: false,
  onRequestStop: vi.fn(),
  onCancelStop: vi.fn(),
  onStop: vi.fn(),
};

describe("SessionPill", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    vi.setSystemTime(NOW);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("sem sessão não aparece", () => {
    const { container } = render(<SessionPill {...base} startedAtMs={null} />);
    expect(container.querySelector(".session-pill")).toBeNull();
  });

  it("mostra o tempo decorrido desde o início da sessão", () => {
    render(<SessionPill {...base} startedAtMs={NOW - 65_000} />);
    expect(screen.getByText("01:05")).toBeDefined();
  });

  it("o relógio anda a cada segundo e não zera ao pausar (tempo é da sessão)", () => {
    render(<SessionPill {...base} startedAtMs={NOW - 5_000} />);
    expect(screen.getByText("00:05")).toBeDefined();

    act(() => {
      vi.advanceTimersByTime(2_000);
    });
    expect(screen.getByText("00:07")).toBeDefined();
  });

  it("clicar na pílula pede o encerramento (não encerra sozinha)", () => {
    const onRequestStop = vi.fn();
    const onStop = vi.fn();
    render(
      <SessionPill
        {...base}
        onRequestStop={onRequestStop}
        onStop={onStop}
        startedAtMs={NOW - 1_000}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /encerrar/i }));
    expect(onRequestStop).toHaveBeenCalledTimes(1);
    expect(onStop).not.toHaveBeenCalled();
  });

  it("confirmação: 'Sim' encerra e 'Não' desiste", () => {
    const onStop = vi.fn();
    const onCancelStop = vi.fn();
    render(
      <SessionPill
        {...base}
        confirming
        onStop={onStop}
        onCancelStop={onCancelStop}
        startedAtMs={NOW - 1_000}
      />,
    );

    expect(screen.getByText("Encerrar sessão?")).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Sim" }));
    expect(onStop).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Não" }));
    expect(onCancelStop).toHaveBeenCalledTimes(1);
  });

  it("durante a operação os botões da confirmação ficam desabilitados", () => {
    render(
      <SessionPill {...base} confirming busy startedAtMs={NOW - 1_000} />,
    );
    expect(screen.getByRole("button", { name: "Sim" })).toHaveProperty(
      "disabled",
      true,
    );
    expect(screen.getByRole("button", { name: "Não" })).toHaveProperty(
      "disabled",
      true,
    );
  });
});
