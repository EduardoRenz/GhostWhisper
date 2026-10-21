import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DwellProvider } from "../../components/dwell/DwellProvider";
import { ActionBar } from "./ActionBar";
import type { SessionChannel } from "../../core/domain/transcript";
import { IDLE_RECORDING } from "../../core/domain/recording";

const show = vi.hoisted(() => vi.fn(async () => undefined));

vi.mock("../../adapters/ipc/dwell", () => ({
  dwellShow: show,
  dwellHide: vi.fn(async () => undefined),
  dwellResult: vi.fn(async () => undefined),
  dwellContext: vi.fn(async () => 500),
}));

const CHANNELS: SessionChannel[] = ["microphone", "system"];

const base = {
  channels: CHANNELS,
  startedAtMs: Date.now(),
  busy: false,
  onStart: vi.fn(),
  onPause: vi.fn(),
  onResume: vi.fn(),
  onStop: vi.fn(),
  onChannelsChange: vi.fn(),
  onStartDrag: vi.fn(),
  startAccelerator: "CommandOrControl+Shift+S",
  askAccelerator: "CommandOrControl+K",
  stealth: false,
  onToggleStealth: vi.fn(),
  onOpenSettings: vi.fn(),
  onOpenTranscription: vi.fn(),
  onOpenSummary: vi.fn(),
    onOpenAsk: vi.fn(),
  suggestionsOpen: false,
  onToggleSuggestions: vi.fn(),
  historyOpen: false,
  onToggleHistory: vi.fn(),
  onHide: vi.fn(),
  recording: IDLE_RECORDING,
  recordingBusy: false,
  recordingCapabilities: { available: true, encoder: "x264", reason: null },
  onToggleRecording: vi.fn(),
  onPickRecordingTarget: vi.fn(),
};

const button = (name: string) =>
  screen.getByRole("button", { name }) as HTMLButtonElement;

function Harness() {
  return (
    <DwellProvider enabled durationMs={500} active>
      <ActionBar {...base} listening paused={false} />
    </DwellProvider>
  );
}

describe("ActionBar — drill-hover opt-in", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    show.mockClear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("só Resumo e Sugestões são gatilhos de confirmação", () => {
    render(<Harness />);

    expect(button("Resumo").getAttribute("aria-haspopup")).toBe("dialog");
    expect(button("Sugestões").getAttribute("aria-haspopup")).toBe("dialog");
    expect(button("Transcrição").getAttribute("aria-haspopup")).toBeNull();
    expect(button("Histórico").getAttribute("aria-haspopup")).toBeNull();
  });

  it("permanecer em Resumo abre a confirmação", () => {
    render(<Harness />);

    fireEvent.pointerOver(button("Resumo"));
    act(() => vi.advanceTimersByTime(500));

    expect(show).toHaveBeenCalledTimes(1);
    expect(show).toHaveBeenCalledWith(
      expect.objectContaining({ width: 0, height: 0 }),
      500,
    );
  });

  it("clique em Sugestões alterna na hora, sem confirmação", () => {
    render(<Harness />);

    fireEvent.pointerOver(button("Sugestões"));
    fireEvent.click(button("Sugestões"));

    expect(base.onToggleSuggestions).toHaveBeenCalledTimes(1);
    expect(show).not.toHaveBeenCalled();
  });
});
