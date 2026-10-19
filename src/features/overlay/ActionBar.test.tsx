import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ActionBar } from "./ActionBar";
import type { SessionChannel } from "../../core/domain/transcript";

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
  stealth: false,
  onToggleStealth: vi.fn(),
  onOpenSettings: vi.fn(),
  onOpenTranscription: vi.fn(),
  onOpenSummary: vi.fn(),
  suggestionsOpen: false,
  onToggleSuggestions: vi.fn(),
  historyOpen: false,
  onToggleHistory: vi.fn(),
  onHide: vi.fn(),
};

const button = (name: string) =>
  screen.getByRole("button", { name }) as HTMLButtonElement;

describe("ActionBar — notch em dois estados", () => {
  it("repouso: marca, ação esmeralda, Histórico, ocultar e engrenagem (última)", () => {
    const { container } = render(
      <ActionBar {...base} listening={false} paused={false} />,
    );

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

    // Atalho em badges.
    const badges = document.querySelectorAll(".shortcut-badge");
    expect([...badges].map((badge) => badge.textContent)).toEqual([
      "Ctrl",
      "Shift",
      "S",
    ]);
  });

  it("repouso: a ação verde inicia com os canais padrão", () => {
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

  it("menu de canais troca ao vivo e oferece Nova sessão", () => {
    const onChannelsChange = vi.fn();
    const onStart = vi.fn();
    const { rerender } = render(
      <ActionBar
        {...base}
        onChannelsChange={onChannelsChange}
        onStart={onStart}
        listening
        paused={false}
      />,
    );

    fireEvent.click(button("Canais de áudio"));
    fireEvent.click(screen.getByRole("menuitemradio", { name: "Sistema" }));
    expect(onChannelsChange).toHaveBeenCalledWith(["system"]);

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
    fireEvent.click(screen.getByRole("menuitem", { name: "Nova sessão" }));
    expect(onStart).toHaveBeenCalledWith(["system"]);
  });
});
