import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const sessionSetLanguage = vi.fn().mockResolvedValue(undefined);
const reload = vi.fn().mockResolvedValue(undefined);

vi.mock("../../adapters/ipc/session", () => ({
  sessionSetLanguage: (...args: unknown[]) => sessionSetLanguage(...args),
}));

vi.mock("../../core/application/useSettings", () => ({
  useSettings: () => ({ settings: { source_language: "auto" }, reload }),
}));

import { LanguageBar } from "./LanguageBar";

const select = () =>
  screen.getByRole("combobox", {
    name: "Idioma da transcrição",
  }) as HTMLSelectElement;

describe("LanguageBar (idioma em tempo real)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("reflete o idioma atual e aplica a troca ao vivo", async () => {
    render(<LanguageBar />);
    expect(select().value).toBe("auto");

    fireEvent.change(select(), { target: { value: "en" } });

    expect(sessionSetLanguage).toHaveBeenCalledWith("en");
    await vi.waitFor(() => expect(reload).toHaveBeenCalled());
  });

  it("não tem transporte (o play/pause vive no notch)", () => {
    render(<LanguageBar />);
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.getByText("Idioma")).toBeDefined();
  });
});
