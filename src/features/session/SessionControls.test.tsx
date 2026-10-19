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
    expect(screen.queryByRole("button", { name: "Retomar" })).toBeNull();

    fireEvent.change(channelsSelect(), { target: { value: "system" } });
    fireEvent.click(button("Iniciar escuta"));

    expect(onStart).toHaveBeenCalledWith(["system"]);
    // Sem sessão não há o que trocar ao vivo.
    expect(base.onChannelsChange).not.toHaveBeenCalled();
  });

  it("gravando: o toggle único pausa e não há botão de encerrar", () => {
    const onPause = vi.fn();
    render(
      <SessionControls {...base} onPause={onPause} listening paused={false} />,
    );

    expect(button("Pausar").disabled).toBe(false);
    // O stop saiu do transporte: encerrar é só a pílula de sessão (BR-022).
    expect(screen.queryByRole("button", { name: /encerrar/i })).toBeNull();
    expect(screen.queryByRole("button", { name: "Retomar" })).toBeNull();

    fireEvent.click(button("Pausar"));
    expect(onPause).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "Nova sessão" })).toBeDefined();
  });

  it("pausado: o mesmo controle retoma (mesma sessão)", () => {
    const onResume = vi.fn();
    render(<SessionControls {...base} onResume={onResume} listening paused />);

    expect(button("Retomar").disabled).toBe(false);
    expect(screen.queryByRole("button", { name: "Pausar" })).toBeNull();

    fireEvent.click(button("Retomar"));
    expect(onResume).toHaveBeenCalledTimes(1);
  });

  it("o toggle único não usa rótulo de texto (só ícone)", () => {
    const { rerender } = render(
      <SessionControls {...base} listening paused={false} />,
    );
    expect(button("Pausar").textContent).toBe("");

    rerender(<SessionControls {...base} listening paused />);
    expect(button("Retomar").textContent).toBe("");
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
    expect(screen.getByRole("button", { name: "Nova sessão" })).toHaveProperty(
      "disabled",
      true,
    );
  });
});
