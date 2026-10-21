import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
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
  onChannelMenuOpenChange: vi.fn(),
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

describe("ActionBar — notch em dois estados", () => {
  it("repouso: lupa, ação verde, Histórico, ocultar e engrenagem (última)", () => {
    const { container } = render(
      <ActionBar {...base} listening={false} paused={false} />,
    );

    // A lupa (barra de pesquisa) existe **independente** da sessão (BR-052).
    expect(button("Perguntar ao GhostWhisper")).toBeDefined();
    expect(button("Começar a ouvir")).toBeDefined();
    expect(button("Histórico")).toBeDefined();
    expect(button("Ocultar overlay")).toBeDefined();
    expect(button("Configurações")).toBeDefined();

    // Nada de sessão em repouso.
    expect(screen.queryByRole("button", { name: "Pausar" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Transcrição" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Sugestões" })).toBeNull();

    // A engrenagem é o ÚLTIMO botão redondo da direita.
    const trailing = container.querySelectorAll(
      ".overlay__group--end .icon-button",
    );
    expect(
      [...trailing].map((button) => button.getAttribute("aria-label")),
    ).toEqual(["Ocultar overlay", "Histórico", "Configurações"]);
    expect(trailing[trailing.length - 1]?.getAttribute("aria-label")).toBe(
      "Configurações",
    );

    // O atalho da lupa aparece em badges (Ctrl K).
    const lupa = button("Perguntar ao GhostWhisper");
    expect(lupa.textContent).toContain("K");
  });

  it("repouso: o microfone inicia com os canais padrão", () => {
    const onStart = vi.fn();
    render(
      <ActionBar
        {...base}
        onStart={onStart}
        listening={false}
        paused={false}
      />,
    );
    fireEvent.click(button("Começar a ouvir"));
    expect(onStart).toHaveBeenCalledWith(["microphone", "system"]);
  });

  it("ativo: switcher (Transcrição/Resumo/Sugestões), pause, timer e ações", () => {
    render(<ActionBar {...base} listening paused={false} />);

    expect(button("Transcrição")).toBeDefined();
    expect(button("Resumo")).toBeDefined();
    expect(button("Pausar")).toBeDefined();
    expect(button("Canais de áudio")).toBeDefined();
    expect(button("Sugestões")).toBeDefined();
    expect(button("Configurações")).toBeDefined();
    expect(screen.getByTestId("notch-waveform")).toBeDefined();

    // Sugestões fica no switcher, ao lado de Resumo; Histórico vive no trailing.
    const suggestions = button("Sugestões");
    const summary = button("Resumo");
    expect(suggestions.closest(".notch-tabs")).not.toBeNull();
    expect(suggestions.closest(".notch-tabs")).toBe(summary.closest(".notch-tabs"));
    expect(button("Histórico").closest(".notch-tabs")).toBeNull();
    expect(button("Histórico").closest(".overlay__group--end")).not.toBeNull();

    // Sem a ação verde quando já há sessão.
    expect(screen.queryByRole("button", { name: "Começar a ouvir" })).toBeNull();
  });

  it("Sugestões alterna a janela e só existe com sessão iniciada", () => {
    const onToggleSuggestions = vi.fn();
    const { rerender } = render(
      <ActionBar
        {...base}
        onToggleSuggestions={onToggleSuggestions}
        listening
        paused={false}
      />,
    );

    fireEvent.click(button("Sugestões"));
    expect(onToggleSuggestions).toHaveBeenCalledTimes(1);

    rerender(<ActionBar {...base} listening={false} paused={false} />);
    expect(screen.queryByRole("button", { name: "Sugestões" })).toBeNull();
  });

  it("Sugestões fica destacado enquanto a janela está aberta", () => {
    render(
      <ActionBar {...base} suggestionsOpen listening paused={false} />,
    );
    expect(button("Sugestões").getAttribute("aria-pressed")).toBe("true");
    expect(
      button("Sugestões").classList.contains("notch-tabs__item--active"),
    ).toBe(true);
  });

  it("pausado: o transporte vira Retomar", () => {
    const onResume = vi.fn();
    render(
      <ActionBar
        {...base}
        onResume={onResume}
        listening
        paused
      />,
    );
    expect(button("Retomar").disabled).toBe(false);
    expect(screen.queryByRole("button", { name: "Pausar" })).toBeNull();
    fireEvent.click(button("Retomar"));
    expect(onResume).toHaveBeenCalledTimes(1);
  });

  it("Transcrição e Resumo abrem o painel nas abas; Histórico alterna", () => {
    const onOpenTranscription = vi.fn();
    const onOpenSummary = vi.fn();
    const onToggleHistory = vi.fn();
    render(
      <ActionBar
        {...base}
        onOpenTranscription={onOpenTranscription}
        onOpenSummary={onOpenSummary}
        onToggleHistory={onToggleHistory}
        listening
        paused={false}
      />,
    );

    fireEvent.click(button("Transcrição"));
    fireEvent.click(button("Resumo"));
    fireEvent.click(button("Histórico"));

    expect(onOpenTranscription).toHaveBeenCalledTimes(1);
    expect(onOpenSummary).toHaveBeenCalledTimes(1);
    expect(onToggleHistory).toHaveBeenCalledTimes(1);
  });

  it("a lupa abre a barra de pesquisa nos dois estados", () => {
    const onOpenAsk = vi.fn();
    const { rerender } = render(
      <ActionBar {...base} onOpenAsk={onOpenAsk} listening={false} paused={false} />,
    );

    // Em repouso (sem transcrição) a lupa já está lá.
    fireEvent.click(button("Perguntar ao GhostWhisper"));
    expect(onOpenAsk).toHaveBeenCalledTimes(1);

    // E continua no estado ativo.
    onOpenAsk.mockClear();
    rerender(
      <ActionBar {...base} onOpenAsk={onOpenAsk} listening paused={false} />,
    );
    fireEvent.click(button("Perguntar ao GhostWhisper"));
    expect(onOpenAsk).toHaveBeenCalledTimes(1);
  });

  it("o switcher ativo não tem mais o item Perguntar (a lupa vive na barra)", () => {
    render(<ActionBar {...base} listening paused={false} />);
    expect(screen.queryByRole("button", { name: "Perguntar" })).toBeNull();
    expect(button("Transcrição")).toBeDefined();
  });

  it("menu de canais troca ao vivo, começa a capturar na sessão e avisa o overlay", () => {
    const onChannelsChange = vi.fn();
    const onStart = vi.fn();
    const onChannelMenuOpenChange = vi.fn();
    const { rerender } = render(
      <ActionBar
        {...base}
        onChannelsChange={onChannelsChange}
        onStart={onStart}
        onChannelMenuOpenChange={onChannelMenuOpenChange}
        listening
        paused={false}
      />,
    );

    fireEvent.click(button("Canais de áudio"));
    expect(onChannelMenuOpenChange).toHaveBeenLastCalledWith(true);
    fireEvent.click(screen.getByRole("menuitemradio", { name: "Sistema" }));
    expect(onChannelsChange).toHaveBeenCalledWith(["system"]);
    expect(onChannelMenuOpenChange).toHaveBeenLastCalledWith(false);

    rerender(
      <ActionBar
        {...base}
        channels={["system"]}
        onStart={onStart}
        listening
        paused={false}
      />,
    );
    fireEvent.click(button("Canais de áudio"));
    fireEvent.click(screen.getByRole("menuitem", { name: "Começar a capturar" }));
    expect(onStart).toHaveBeenCalledWith(["system"]);
  });
});
