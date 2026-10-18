import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SessionControls } from "./SessionControls";
import type { SessionChannel } from "../../core/domain/transcript";

const BOTH: SessionChannel[] = ["microphone", "system"];

const base = {
  channels: BOTH,
  busy: false,
  onStart: vi.fn(),
  onPause: vi.fn(),
  onResume: vi.fn(),
  onStop: vi.fn(),
  onChannelsChange: vi.fn(),
};

const channelsSelect = (): HTMLSelectElement =>
  screen.getByRole("combobox") as HTMLSelectElement;

describe("SessionControls", () => {
  it("parado: só oferece iniciar", () => {
    render(<SessionControls {...base} listening={false} paused={false} />);

    expect(screen.getByRole("button", { name: "Iniciar" })).toBeDefined();
    expect(screen.queryByRole("button", { name: "Pausar" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Parar" })).toBeNull();
  });

  it("inicia com os canais escolhidos", () => {
    const onStart = vi.fn();
    render(
      <SessionControls
        {...base}
        onStart={onStart}
        listening={false}
        paused={false}
      />,
    );

    fireEvent.change(channelsSelect(), { target: { value: "system" } });
    fireEvent.click(screen.getByRole("button", { name: "Iniciar" }));

    expect(onStart).toHaveBeenCalledWith(["system"]);
    // Sem sessão não há o que trocar ao vivo.
    expect(base.onChannelsChange).not.toHaveBeenCalled();
  });

  it("gravando: pausar, nova sessão e parar", () => {
    const onPause = vi.fn();
    render(
      <SessionControls
        {...base}
        onPause={onPause}
        listening
        paused={false}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Pausar" }));
    expect(onPause).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "Nova sessão" })).toBeDefined();
    expect(screen.getByRole("button", { name: "Parar" })).toBeDefined();
    expect(screen.queryByRole("button", { name: "Retomar" })).toBeNull();
  });

  it("pausado: oferece retomar (e não pausar de novo)", () => {
    const onResume = vi.fn();
    render(
      <SessionControls {...base} onResume={onResume} listening paused />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Retomar" }));
    expect(onResume).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: "Pausar" })).toBeNull();
    // A sessão segue aberta enquanto pausada: dá para continuar ou encerrar.
    expect(screen.getByRole("button", { name: "Nova sessão" })).toBeDefined();
    expect(screen.getByRole("button", { name: "Parar" })).toBeDefined();
  });

  // --- Troca de canais (sempre permitida) ---------------------------------

  it("deixa trocar os canais com a sessão gravando", () => {
    const onChannelsChange = vi.fn();
    render(
      <SessionControls
        {...base}
        onChannelsChange={onChannelsChange}
        listening
        paused={false}
      />,
    );

    const select = channelsSelect();
    expect(select.disabled).toBe(false);

    fireEvent.change(select, { target: { value: "system" } });

    expect(onChannelsChange).toHaveBeenCalledWith(["system"]);
  });

  it("deixa trocar os canais com a sessão pausada", () => {
    const onChannelsChange = vi.fn();
    render(
      <SessionControls
        {...base}
        onChannelsChange={onChannelsChange}
        listening
        paused
      />,
    );

    fireEvent.change(channelsSelect(), { target: { value: "microphone" } });

    expect(onChannelsChange).toHaveBeenCalledWith(["microphone"]);
  });

  it("reflete os canais que a sessão está captando", () => {
    const { rerender } = render(
      <SessionControls {...base} listening channels={["microphone"]} paused={false} />,
    );
    expect(channelsSelect().value).toBe("microphone");

    rerender(
      <SessionControls {...base} listening channels={["system"]} paused={false} />,
    );
    expect(channelsSelect().value).toBe("system");
  });

  it("nova sessão reaproveita os canais em uso", () => {
    const onStart = vi.fn();
    render(
      <SessionControls
        {...base}
        onStart={onStart}
        listening
        channels={["system"]}
        paused={false}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Nova sessão" }));

    expect(onStart).toHaveBeenCalledWith(["system"]);
  });
});
