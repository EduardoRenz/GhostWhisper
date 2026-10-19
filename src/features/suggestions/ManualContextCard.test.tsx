import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ManualContextCard } from "./ManualContextCard";

const context = {
  session_id: "s1",
  speaker: "ME" as const,
  text: "Primeiro eu começo pela camada de API.",
  at_ms: 47_000,
};

const started = new Date(2026, 9, 19, 10, 42, 15).getTime();

describe("ManualContextCard", () => {
  it("mostra falante, hora, trecho e o botão de gerar", () => {
    const onGenerate = vi.fn();
    const onClear = vi.fn();
    render(
      <ManualContextCard
        context={context}
        startedAtMs={started}
        generating={false}
        generated={false}
        onGenerate={onGenerate}
        onClear={onClear}
      />,
    );

    expect(screen.getByText("Contexto manual")).toBeDefined();
    expect(screen.getByText("Microfone")).toBeDefined();
    expect(screen.getByText("10:43:02")).toBeDefined();
    expect(screen.getByText(context.text)).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: /Gerar sugestão manual/ }));
    expect(onGenerate).toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Remover contexto manual" }));
    expect(onClear).toHaveBeenCalled();
  });

  it("desabilita o botão e avisa enquanto uma geração está em andamento", () => {
    render(
      <ManualContextCard
        context={context}
        startedAtMs={started}
        generating
        generated={false}
        onGenerate={vi.fn()}
        onClear={vi.fn()}
      />,
    );

    const button = screen.getByRole("button", {
      name: /Aguardando a sugestão em andamento/,
    });
    expect(button).toBeDefined();
    expect(button.hasAttribute("disabled")).toBe(true);
  });
});
