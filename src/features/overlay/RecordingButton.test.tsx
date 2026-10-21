import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { IDLE_RECORDING } from "../../core/domain/recording";
import { RecordingButton } from "./RecordingButton";

const CAPABILITIES = {
  available: true,
  encoder: "x264",
  reason: null,
};

describe("RecordingButton — botão de câmera", () => {
  it("desligado: câmera neutra e aria-pressed falso", () => {
    render(
      <RecordingButton
        state={IDLE_RECORDING}
        capabilities={CAPABILITIES}
        busy={false}
        onToggle={vi.fn()}
        onPick={vi.fn()}
      />,
    );
    const button = screen.getByRole("button", { name: /Gravar esta sessão/ });
    expect(button.getAttribute("aria-pressed")).toBe("false");
    expect(button.className).not.toContain("--live");
  });

  it("gravando: fica vermelho, aria-pressed e ponto REC", () => {
    const { container } = render(
      <RecordingButton
        state={{ ...IDLE_RECORDING, status: "recording" }}
        capabilities={CAPABILITIES}
        busy={false}
        onToggle={vi.fn()}
        onPick={vi.fn()}
      />,
    );
    const button = screen.getByRole("button", { name: "Pausar gravação" });
    expect(button.getAttribute("aria-pressed")).toBe("true");
    expect(container.querySelector(".notch-tabs__rec-dot")).not.toBeNull();
  });

  it("pausado: vermelho vazado e rótulo de retomar", () => {
    const { container } = render(
      <RecordingButton
        state={{ ...IDLE_RECORDING, status: "paused" }}
        capabilities={CAPABILITIES}
        busy={false}
        onToggle={vi.fn()}
        onPick={vi.fn()}
      />,
    );
    const button = screen.getByRole("button", { name: "Retomar gravação" });
    expect(button.className).toContain("notch-tabs__recording--paused");
    expect(container.querySelector(".notch-tabs__rec-dot")).toBeNull();
  });

  it("indisponível: desabilitado com o motivo no título", () => {
    render(
      <RecordingButton
        state={IDLE_RECORDING}
        capabilities={{
          available: false,
          encoder: "x264",
          reason: "ffmpeg não encontrado",
        }}
        busy={false}
        onToggle={vi.fn()}
        onPick={vi.fn()}
      />,
    );
    const button = screen.getByRole("button", { name: "ffmpeg não encontrado" });
    expect((button as HTMLButtonElement).disabled).toBe(true);
  });

  it("clique dispara o toggle", () => {
    const onToggle = vi.fn();
    render(
      <RecordingButton
        state={IDLE_RECORDING}
        capabilities={CAPABILITIES}
        busy={false}
        onToggle={onToggle}
        onPick={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Gravar esta sessão/ }));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });
});
