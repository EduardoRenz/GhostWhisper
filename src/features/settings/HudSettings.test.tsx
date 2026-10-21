import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { defaultSettings } from "../../core/domain/settings";
import { HudSettings } from "./HudSettings";

function setup(overrides: Partial<ReturnType<typeof defaultSettings>> = {}) {
  const settings = { ...defaultSettings(), ...overrides };
  const update = vi.fn();
  render(<HudSettings settings={settings} update={update} />);
  return { update };
}

describe("HudSettings (aba Geral & HUD)", () => {
  it("mostra o cabeçalho da seção e o rótulo vivo da âncora", () => {
    setup({ notch_anchor: "top_right" });
    expect(screen.getByText("Comportamento do HUD & Janela")).toBeDefined();
    expect(screen.getByText("HUD Anchor: Canto Sup. Dir.")).toBeDefined();
  });

  it("mostra os quatro cards do mockup + privacidade", () => {
    setup();
    expect(screen.getByText("Opacidade das Janelas")).toBeDefined();
    expect(screen.getByText("Posição de Ancoragem do Notch")).toBeDefined();
    expect(screen.getByText("Expandir Notch por Dwell (Hover)")).toBeDefined();
    expect(screen.getByText("Tamanho / Densidade do Notch")).toBeDefined();
    expect(screen.getByText("Privacidade")).toBeDefined();
  });

  it("muda a opacidade das janelas e atualiza o badge", () => {
    const { update } = setup({ overlay_opacity: 0.75 });
    const slider = screen.getByLabelText("Opacidade das janelas") as HTMLInputElement;
    expect(slider.value).toBe("75");
    expect(slider.min).toBe("20");
    expect(slider.max).toBe("100");
    expect(screen.getByText("75%")).toBeDefined();

    fireEvent.change(slider, { target: { value: "40" } });
    expect(update).toHaveBeenCalledWith("overlay_opacity", 0.4);
  });

  it("troca a ancoragem do notch", () => {
    const { update } = setup();
    fireEvent.click(screen.getByRole("button", { name: "Canto Sup. Dir." }));
    expect(update).toHaveBeenCalledWith("notch_anchor", "top_right");
    fireEvent.click(screen.getByRole("button", { name: "Flutuante Livre" }));
    expect(update).toHaveBeenCalledWith("notch_anchor", "free");
  });

  it("troca a densidade do notch", () => {
    const { update } = setup();
    fireEvent.click(screen.getByRole("button", { name: "Expandido" }));
    expect(update).toHaveBeenCalledWith("hud_density", "expanded");
    fireEvent.click(screen.getByRole("button", { name: "Compacto" }));
    expect(update).toHaveBeenCalledWith("hud_density", "compact");
  });

  it("configura o dwell-hover com a faixa do mockup", () => {
    const { update } = setup({ dwell_enabled: true, dwell_ms: 500 });
    const toggle = screen.getByRole("checkbox", {
      name: "Expandir notch por dwell",
    }) as HTMLInputElement;
    expect(toggle.checked).toBe(true);

    const slider = screen.getByLabelText(
      "Tempo de dwell em milissegundos",
    ) as HTMLInputElement;
    expect(slider.value).toBe("500");
    expect(slider.min).toBe("100");
    expect(slider.max).toBe("1500");
    expect(slider.disabled).toBe(false);

    fireEvent.change(slider, { target: { value: "1200" } });
    expect(update).toHaveBeenCalledWith("dwell_ms", 1200);

    fireEvent.click(toggle);
    expect(update).toHaveBeenCalledWith("dwell_enabled", false);
  });

  it("desabilita o slider de dwell quando o recurso está desligado", () => {
    setup({ dwell_enabled: false });
    expect(
      (screen.getByLabelText("Tempo de dwell em milissegundos") as HTMLInputElement)
        .disabled,
    ).toBe(true);
  });

  it("marca o stealth no card de privacidade", () => {
    const { update } = setup({ stealth: false });
    fireEvent.click(screen.getByRole("checkbox", { name: "Modo stealth" }));
    expect(update).toHaveBeenCalledWith("stealth", true);
  });
});
