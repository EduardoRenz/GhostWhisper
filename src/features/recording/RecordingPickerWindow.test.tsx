import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type {
  RecordingState,
  RecordingTarget,
} from "../../core/domain/recording";
import {
  recordingPickerClose,
  recordingStartWith,
  recordingState,
  recordingTargets,
} from "../../adapters/ipc/recording";
import { RecordingPickerWindow } from "./RecordingPickerWindow";

vi.mock("../../adapters/ipc/recording", () => ({
  recordingState: vi.fn(),
  recordingTargets: vi.fn(),
  recordingStartWith: vi.fn(async () => undefined),
  recordingPickerClose: vi.fn(async () => undefined),
}));

const IDLE: RecordingState = {
  status: "idle",
  kind: null,
  session_id: null,
  target_label: null,
  elapsed_ms: 0,
  size_bytes: 0,
};

const TARGETS: RecordingTarget[] = [
  {
    kind: "screen",
    label: "Tela inteira",
    width: 1920,
    height: 1080,
    x: 0,
    y: 0,
  },
  {
    kind: "window",
    id: 42,
    label: "Reunião — Chrome",
    width: 1280,
    height: 720,
  },
];

describe("RecordingPickerWindow — seletor de janelas", () => {
  beforeEach(() => {
    vi.mocked(recordingState).mockResolvedValue(IDLE);
    vi.mocked(recordingTargets).mockResolvedValue(TARGETS);
    vi.mocked(recordingStartWith).mockClear();
    vi.mocked(recordingPickerClose).mockClear();
  });

  it("lista a tela inteira e as janelas", async () => {
    render(<RecordingPickerWindow />);

    expect(await screen.findByText("Tela inteira")).toBeDefined();
    expect(screen.getByText("Reunião — Chrome")).toBeDefined();
    expect(screen.getByText("1280×720")).toBeDefined();
  });

  it("escolher uma janela inicia a gravação naquele alvo", async () => {
    render(<RecordingPickerWindow />);

    fireEvent.click(await screen.findByText("Reunião — Chrome"));
    await waitFor(() => {
      expect(recordingStartWith).toHaveBeenCalledWith(TARGETS[1]);
    });
    expect(recordingPickerClose).toHaveBeenCalled();
  });

  it("bloqueia a troca de alvo com gravação em andamento", async () => {
    vi.mocked(recordingState).mockResolvedValue({
      ...IDLE,
      status: "recording",
      target_label: "Tela inteira",
    });
    render(<RecordingPickerWindow />);

    expect(
      await screen.findByText(/Já existe uma gravação em andamento/),
    ).toBeDefined();
    const target = screen.getByText("Tela inteira");
    const button = target.closest("button") as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });
});
