import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SuggestionsActivationBlock } from "./SuggestionsActivationBlock";
import {
  defaultSettings,
  type Settings,
} from "../../core/domain/settings";

vi.mock("../../core/application/useClassifierModels", () => ({
  useClassifierModels: () => ({
    catalog: null,
    backend: {
      laya_feature: false,
      model_active: false,
      model_configured: false,
      model_bundled: false,
    },
    recommended: null,
    installed: [],
    active: null,
    loading: false,
    busy: false,
    error: null,
    progress: null,
    refresh: vi.fn(async () => {}),
    install: vi.fn(async () => {}),
    activate: vi.fn(async () => {}),
    remove: vi.fn(async () => {}),
  }),
  progressLabel: () => null,
  progressPercent: () => null,
}));

vi.mock("../../core/application/useSessionState", () => ({
  useSessionState: () => ({
    state: { listening: true, paused: false, session_id: "s1" },
  }),
}));

function Harness({ initial }: { initial?: Settings }) {
  const [settings, setSettings] = useState<Settings>(initial ?? defaultSettings());
  const update = <K extends keyof Settings>(field: K, value: Settings[K]) =>
    setSettings((previous) => ({ ...previous, [field]: value }));
  return <SuggestionsActivationBlock settings={settings} update={update} />;
}

describe("SuggestionsActivationBlock", () => {
  it("mostra os dois cards e a estratégia ativa", () => {
    render(<Harness />);
    expect(screen.getByText("Modo de ativação de sugestões")).toBeDefined();
    expect(screen.getByText("Modelo de classificação")).toBeDefined();
    expect(screen.getByText("Sugestão por tempo")).toBeDefined();
    expect(screen.getByText("Ativa")).toBeDefined();
  });

  it("troca para o classificador e liga o interruptor mestre", () => {
    const initial: Settings = {
      ...defaultSettings(),
      suggestion_auto_refresh: false,
    };
    render(<Harness initial={initial} />);
    fireEvent.click(screen.getByText("Modelo de classificação"));

    const toggle = screen.getByLabelText("Sugestões automáticas") as HTMLInputElement;
    expect(toggle.checked).toBe(true);
    expect(screen.getByText("Ativa")).toBeDefined();
  });

  it("oferece só Laya multilingual (English removido)", () => {
    render(<Harness />);
    const select = screen.getByLabelText("Modelo selecionado") as HTMLSelectElement;
    const values = Array.from(select.options).map((option) => option.value);
    expect(values).toEqual(["laya", "heuristic"]);
    expect(values).not.toContain("laya_english");

    fireEvent.change(select, { target: { value: "laya" } });
    expect(select.value).toBe("laya");
  });

  it("troca a sensibilidade", () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "Alta" }));
    expect(screen.getByRole("button", { name: "Alta" }).className).toContain(
      "is-active",
    );
  });

  it("troca o escopo do classificador para adaptativo", () => {
    render(<Harness />);
    const select = screen.getByLabelText("Quando sugerir") as HTMLSelectElement;
    expect(select.value).toBe("adaptive");
    expect(screen.getByRole("option", { name: "Só THEY (perguntas do outro)" })).toBeDefined();
    fireEvent.change(select, { target: { value: "they_only" } });
    expect(select.value).toBe("they_only");
    fireEvent.change(select, { target: { value: "blocks" } });
    expect(select.value).toBe("blocks");
  });

  it("troca a cadência do card de tempo", () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "30s" }));
    expect(screen.getByRole("button", { name: "30s" }).className).toContain(
      "is-active",
    );
  });

  it("abre o avançado e mostra os campos", () => {
    render(<Harness />);
    fireEvent.click(screen.getByText("Modelo de classificação"));
    fireEvent.click(screen.getByRole("button", { name: /Avançado/ }));
    expect(screen.getByText("Cooldown (s)")).toBeDefined();
    expect(screen.getByText("Categorias que geram sugestão")).toBeDefined();
    expect(screen.getByText("Testar classificador")).toBeDefined();
  });
});
