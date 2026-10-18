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
  onRequestStop: vi.fn(),
  onChannelsChange: vi.fn(),
};

const button = (name: string): HTMLButtonElement =>
  screen.getByRole("button", { name }) as HTMLButtonElement;

const channelsSelect = (): HTMLSelectElement =>
  screen.getByRole("combobox") as HTMLSelectElement;

describe("SessionControls", () => {
  it("parado: oferece 'Iniciar escuta' com os canais escolhidos", () => {
    const onStart = vi.fn();
    render(
      <SessionControls {...base} onStart={onStart} listening={false} paused={false} />,
    );

    expect(button("Iniciar escuta")).toBeDefined();
    expect(screen.queryByRole("button", { name: "Pausar" })).toBeNull();

    fireEvent.change(channelsSelect(), { target: { value: "system" } });
    fireEvent.click(button("Iniciar escuta"));

    expect(onStart).toHaveBeenCalledWith(["system"]);
    // Sem sessão não há o que trocar ao vivo.
    expect(base.onChannelsChange).not.toHaveBeenCalled();
  });

  it("gravando: play esmaecido, pause e stop ativos", () => {
    const onPause = vi.fn();
    const onRequestStop = vi.fn();
    render(
      <SessionControls
        {...base}
        onPause={onPause}
        onRequestStop={onRequestStop}
        listening
        paused={false}
      />,
    );

    expect(button("Retomar").disabled).toBe(true);
    expect(button("Pausar").disabled).toBe(false);
    expect(button("Encerrar sessão").disabled).toBe(false);

    fireEvent.click(button("Pausar"));
    expect(onPause).toHaveBeenCalledTimes(1);

    fireEvent.click(button("Encerrar sessão"));
    expect(onRequestStop).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "Nova sessão" })).toBeDefined();
  });

  it("pausado: play ativo e pause esmaecido (mesma sessão)", () => {
    const onResume = vi.fn();
    render(
      <SessionControls {...base} onResume={onResume} listening paused />,
    );

    expect(button("Retomar").disabled).toBe(false);
    expect(button("Pausar").disabled).toBe(true);
    expect(button("Encerrar sessão").disabled).toBe(false);

    fireEvent.click(button("Retomar"));
    expect(onResume).toHaveBeenCalledTimes(1);
  });

  it("pílula de transporte não usa rótulo de texto (só ícone)", () => {
    render(<SessionControls {...base} listening paused={false} />);
    expect(button("Pausar").textContent).toBe("");
    expect(button("Encerrar sessão").textContent).toBe("");
  });

  it("deixa trocar os canais com a sessão gravando ou pausada", () => {
    const onChannelsChange = vi.fn();
    const { rerender } = render(
      <SessionControls
        {...base}
        onChannelsChange={onChannelsChange}
        listening
        paused={false}
      />,
    );

    expect(channelsSelect().disabled).toBe(false);
    fireEvent.change(channelsSelect(), { target: { value: "system" } });
    expect(onChannelsChange).toHaveBeenCalledWith(["system"]);

    rerender(
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

  it("desabilita as ações durante uma operação em curso", () => {
    render(<SessionControls {...base} busy listening paused={false} />);
    expect(button("Pausar").disabled).toBe(true);
    expect(button("Encerrar sessão").disabled).toBe(true);
    expect(screen.getByRole("button", { name: "Nova sessão" })).toHaveProperty(
      "disabled",
      true,
    );
  });
});
