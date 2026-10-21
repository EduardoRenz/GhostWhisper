import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { defaultSettings } from "../../core/domain/settings";
import { RecordingSettings } from "./RecordingSettings";

describe("RecordingSettings — aba Gravação", () => {
  it("parte dos padrões da máquina de referência", () => {
    render(<RecordingSettings settings={defaultSettings()} update={vi.fn()} />);

    expect(
      screen
        .getByRole("button", { name: "Vídeo + áudio" })
        .getAttribute("aria-pressed"),
    ).toBe("true");
    expect(
      screen
        .getByRole("button", { name: "Janela da reunião" })
        .getAttribute("aria-pressed"),
    ).toBe("true");
    // A opção ativa precisa parecer selecionada (cor primária), não só ter
    // `aria-pressed`.
    expect(
      screen.getByRole("button", { name: "Vídeo + áudio" }).className,
    ).toContain("settings__segment--active");
    expect(
      screen.getByRole("button", { name: "Somente áudio" }).className,
    ).not.toContain("settings__segment--active");
    expect(
      screen.getByRole("button", { name: "8" }).getAttribute("aria-pressed"),
    ).toBe("true");
    expect(
      screen
        .getByRole("button", { name: "Nativa" })
        .getAttribute("aria-pressed"),
    ).toBe("true");
    expect(screen.getByText("109–219 MB/h")).toBeDefined();
  });

  it("alternar para somente áudio esconde o card de vídeo", () => {
    const update = vi.fn();
    const { rerender } = render(
      <RecordingSettings settings={defaultSettings()} update={update} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Somente áudio" }));
    expect(update).toHaveBeenCalledWith("recording_mode", "audio");

    rerender(
      <RecordingSettings
        settings={{ ...defaultSettings(), recording_mode: "audio" }}
        update={update}
      />,
    );
    expect(screen.queryByText("Vídeo")).toBeNull();
    expect(screen.getByText(/FLAC sem perda/)).toBeDefined();
  });

  it("altera FPS, qualidade e container", () => {
    const update = vi.fn();
    render(<RecordingSettings settings={defaultSettings()} update={update} />);

    fireEvent.click(screen.getByRole("button", { name: "15" }));
    expect(update).toHaveBeenCalledWith("recording_fps", 15);

    fireEvent.click(screen.getByRole("button", { name: "Nítida" }));
    expect(update).toHaveBeenCalledWith("recording_quality", "sharp");

    fireEvent.click(screen.getByRole("button", { name: "MKV" }));
    expect(update).toHaveBeenCalledWith("recording_container", "mkv");
  });

  it("limpa a pasta quando o campo fica vazio", () => {
    const update = vi.fn();
    render(
      <RecordingSettings
        settings={{ ...defaultSettings(), recording_dir: "/tmp/videos" }}
        update={update}
      />,
    );

    const input = screen.getByLabelText("Pasta de destino");
    fireEvent.change(input, { target: { value: "" } });
    expect(update).toHaveBeenCalledWith("recording_dir", null);
  });
});
