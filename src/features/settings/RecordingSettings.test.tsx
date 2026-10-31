import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { RecordingCapabilities } from "../../core/domain/recording";
import { defaultSettings } from "../../core/domain/settings";
import { RecordingSettings } from "./RecordingSettings";

// O adaptador IPC é mockado no arquivo inteiro: por padrão devolve `undefined`
// (fora do Tauri), e o teste de indisponibilidade injeta as capabilities.
const ipc = vi.hoisted(() => ({
  capabilities: undefined as RecordingCapabilities | undefined,
}));

vi.mock("../../adapters/ipc/recording", () => ({
  recordingCapabilities: async () => ipc.capabilities,
}));

afterEach(() => {
  ipc.capabilities = undefined;
});

describe("RecordingSettings — aba Gravação", () => {
  it("sem ffmpeg: avisa em l10n que falta instalar", async () => {
    ipc.capabilities = {
      available: false,
      encoder: "x264",
      reason: "ffmpeg não encontrado — instale o pacote ou defina GHOSTWHISPER_FFMPEG",
      reason_code: "ffmpeg_missing",
    };

    render(<RecordingSettings settings={defaultSettings()} update={vi.fn()} />);

    expect(
      await screen.findByText("Instale o ffmpeg para liberar a gravação"),
    ).toBeDefined();
  });

  it("no Wayland não oferece seguir a janela ativa (portal)", async () => {
    ipc.capabilities = {
      available: true,
      encoder: "x264",
      reason: null,
      reason_code: null,
      follow_supported: false,
      target_mode: "portal",
    };

    render(<RecordingSettings settings={defaultSettings()} update={vi.fn()} />);

    expect(
      await screen.findByText(/o GNOME abre o seletor de janela ou tela/),
    ).toBeDefined();
    expect(screen.queryByRole("button", { name: "Janela da reunião" })).toBeNull();
    expect(screen.queryByText("Seguir a janela ativa")).toBeNull();
    expect(screen.getByText("Escolher no GNOME")).toBeDefined();
  });

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
