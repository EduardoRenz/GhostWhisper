import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { defaultSettings } from "../../core/domain/settings";
import { setUiLanguage } from "../../core/l10n";
import { LanguageSettings } from "./LanguageSettings";

describe("LanguageSettings — idioma da interface", () => {
  it("mostra o idioma atual e as duas opções suportadas", () => {
    const update = vi.fn();
    render(<LanguageSettings settings={defaultSettings()} update={update} />);

    const select = screen.getByLabelText("Idioma da interface") as HTMLSelectElement;
    expect(select.value).toBe("pt");
    expect(screen.getByRole("option", { name: "Português (Brasil)" })).toBeDefined();
    expect(screen.getByRole("option", { name: "English (US)" })).toBeDefined();
  });

  it("troca para inglês sem mexer em outros campos", () => {
    const update = vi.fn();
    render(<LanguageSettings settings={defaultSettings()} update={update} />);

    fireEvent.change(screen.getByLabelText("Idioma da interface"), {
      target: { value: "en" },
    });

    expect(update).toHaveBeenCalledWith("ui_language", "en");
    expect(update).toHaveBeenCalledTimes(1);
  });

  it("respeita o idioma já persistido", () => {
    render(
      <LanguageSettings
        settings={{ ...defaultSettings(), ui_language: "en" }}
        update={vi.fn()}
      />,
    );
    expect(
      (screen.getByLabelText("Idioma da interface") as HTMLSelectElement).value,
    ).toBe("en");
  });

  it("aplica o inglês em tempo real quando o store muda", async () => {
    render(<LanguageSettings settings={defaultSettings()} update={vi.fn()} />);
    expect(screen.getByText("Idioma da Interface")).toBeDefined();

    await act(async () => {
      setUiLanguage("en");
    });

    expect(screen.getByText("Interface Language")).toBeDefined();
    expect(screen.getByLabelText("Interface language")).toBeDefined();
  });
});
