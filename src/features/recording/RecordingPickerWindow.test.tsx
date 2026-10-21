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
  recordingSwitchTarget,
  recordingTargets,
} from "../../adapters/ipc/recording";
import { RecordingPickerWindow } from "./RecordingPickerWindow";

vi.mock("../../adapters/ipc/recording", () => ({
  recordingState: vi.fn(),
  recordingTargets: vi.fn(),
  recordingStartWith: vi.fn(async () => undefined),
  recordingSwitchTarget: vi.fn(async () => undefined),
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
    vi.mocked(recordingSwitchTarget).mockClear();
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

  it("com gravação em andamento, escolher troca o alvo", async () => {
    vi.mocked(recordingState).mockResolvedValue({
      ...IDLE,
      status: "recording",
      target_label: "VS Code",
    });
    render(<RecordingPickerWindow />);

    expect(
      await screen.findByText(/A gravação continua no mesmo arquivo/),
    ).toBeDefined();
    fireEvent.click(screen.getByText("Reunião — Chrome"));
    await waitFor(() => {
      expect(recordingSwitchTarget).toHaveBeenCalledWith(TARGETS[1]);
    });
    expect(recordingStartWith).not.toHaveBeenCalled();
  });
});
