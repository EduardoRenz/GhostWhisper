import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DwellProvider } from "../../components/dwell/DwellProvider";
import { ActionBar } from "./ActionBar";
import type { SessionChannel } from "../../core/domain/transcript";
import { IDLE_RECORDING } from "../../core/domain/recording";

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
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("só Resumo e Sugestões usam o drill-hover", () => {
    render(<Harness />);

    expect(button("Resumo").classList.contains("dwell-target")).toBe(true);
    expect(button("Sugestões").classList.contains("dwell-target")).toBe(true);
    expect(button("Transcrição").classList.contains("dwell-target")).toBe(
      false,
    );
    expect(button("Histórico").classList.contains("dwell-target")).toBe(false);
  });

  it("permanecer em Resumo executa a ação diretamente", () => {
    render(<Harness />);

    fireEvent.pointerOver(button("Resumo"));
    act(() => vi.advanceTimersByTime(500));

    expect(base.onOpenSummary).toHaveBeenCalledTimes(1);
  });

  it("passar do ícone/texto não reinicia o dwell", () => {
    render(<Harness />);
    const resumo = button("Resumo");

    fireEvent.pointerOver(resumo);
    act(() => vi.advanceTimersByTime(250));
    fireEvent.pointerOver(screen.getByText("Resumo"));
    act(() => vi.advanceTimersByTime(250));

    expect(base.onOpenSummary).toHaveBeenCalledTimes(1);
  });

  it("clique em Sugestões alterna na hora e não agenda de novo", () => {
    render(<Harness />);

    fireEvent.pointerOver(button("Sugestões"));
    fireEvent.click(button("Sugestões"));
    act(() => vi.advanceTimersByTime(500));

    expect(base.onToggleSuggestions).toHaveBeenCalledTimes(1);
  });
});
