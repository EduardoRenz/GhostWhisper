import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CadenceSettings } from "./CadenceSettings";
import {
  defaultSettings,
  MAX_SAVED_QUESTIONS,
  type SavedQuestion,
  type Settings,
} from "../../core/domain/settings";

/** Aplica as edições no rascunho, como o `SettingsPage` faz. */
function Harness({ initial }: { initial?: Settings }) {
  const [settings, setSettings] = useState<Settings>(initial ?? defaultSettings());
  const update = <K extends keyof Settings>(field: K, value: Settings[K]) =>
    setSettings((previous) => ({ ...previous, [field]: value }));
  return <CadenceSettings settings={settings} update={update} />;
}

describe("CadenceSettings", () => {
  it("mostra as cadências e as perguntas prontas", () => {
    render(<Harness />);

    expect(screen.getByText("Resumo automático")).toBeDefined();
    expect(screen.getByText("Sugestões automáticas")).toBeDefined();
    expect(screen.getByText("Perguntas prontas")).toBeDefined();
    expect(
      (screen.getByLabelText("Intervalo do resumo") as HTMLSelectElement).value,
    ).toBe("30");
  });

  it("desabilita o intervalo quando a cadência está desligada", () => {
    const initial: Settings = {
      ...defaultSettings(),
      summary_auto_refresh: false,
    };
    render(<Harness initial={initial} />);

    expect(
      (screen.getByLabelText("Intervalo do resumo") as HTMLSelectElement)
        .disabled,
    ).toBe(true);
  });

  it("troca o intervalo do resumo", () => {
    render(<Harness />);
    const select = screen.getByLabelText(
      "Intervalo do resumo",
    ) as HTMLSelectElement;

    fireEvent.change(select, { target: { value: "60" } });
    expect(select.value).toBe("60");
  });

  it("liga e desliga as sugestões automáticas", () => {
    render(<Harness />);
    const toggle = screen.getByRole("checkbox", {
      name: "Sugestões automáticas",
    }) as HTMLInputElement;

    expect(toggle.checked).toBe(true);
    fireEvent.click(toggle);
    expect(toggle.checked).toBe(false);
  });

  it("acrescenta e remove uma pergunta pronta", () => {
    const initial: Settings = {
      ...defaultSettings(),
      saved_questions: [{ id: "q-1", text: "Como otimizar esta consulta?" }],
    };
    render(<Harness initial={initial} />);

    expect(
      (screen.getByLabelText("Pergunta pronta") as HTMLInputElement).value,
    ).toBe("Como otimizar esta consulta?");

    fireEvent.click(
      screen.getByRole("button", { name: "Nova pergunta pronta" }),
    );
    expect(screen.getAllByLabelText("Pergunta pronta")).toHaveLength(2);

    fireEvent.click(
      screen.getAllByRole("button", { name: "Remover pergunta pronta" })[0]!,
    );
    expect(screen.getAllByLabelText("Pergunta pronta")).toHaveLength(1);
  });

  it("não deixa passar do limite de perguntas prontas", () => {
    const questions: SavedQuestion[] = Array.from(
      { length: MAX_SAVED_QUESTIONS },
      (_, index) => ({ id: `q-${index}`, text: `Pergunta ${index}` }),
    );
    render(
      <Harness initial={{ ...defaultSettings(), saved_questions: questions }} />,
    );

    const add = screen.getByRole("button", {
      name: "Nova pergunta pronta",
    }) as HTMLButtonElement;
    expect(add.disabled).toBe(true);
  });
});
