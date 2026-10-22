import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AgentCard } from "./AgentCard";
import type { AssistantMode } from "../../core/domain/assistant";

const mode = (overrides: Partial<AssistantMode> = {}): AssistantMode => ({
  id: "custom-vendas",
  name: "Vendas",
  category: "custom",
  system_prompt: "Critérios de vendas.",
  suggestion_prompt: "Analise a objeção e sugira a próxima fala.",
  summary_prompt: "Extraia pontos-chave, trade-offs e alternativas.",
  context: "",
  icon: "briefcase",
  color: "coral",
  ...overrides,
});

const noop = () => {};

describe("AgentCard", () => {
  it("mostra os painéis dos dois prompts com o texto do agente", () => {
    render(
      <AgentCard
        mode={mode()}
        active={false}
        onEdit={noop}
        onDuplicate={noop}
        onDelete={noop}
      />,
    );

    expect(screen.getByText("Prompt de Sugestão")).toBeDefined();
    expect(screen.getByText("Prompt de Resumo")).toBeDefined();
    expect(screen.getByText(/Analise a objeção/)).toBeDefined();
    expect(screen.getByText(/Extraia pontos-chave/)).toBeDefined();
    // A pill "Markdown AST" do mockup foi removida (revisão de 2026-10-22).
    expect(screen.queryByText("Markdown AST")).toBeNull();
    // Cada painel tem ajuda (BR-081).
    expect(
      screen.getByRole("button", { name: "Ajuda: Prompt de Sugestão" }),
    ).toBeDefined();
    expect(
      screen.getByRole("button", { name: "Ajuda: Prompt de Resumo" }),
    ).toBeDefined();
  });

  it("mostra a badge Ativo só quando é o agente em uso", () => {
    const { container, rerender } = render(
      <AgentCard
        mode={mode()}
        active
        onEdit={noop}
        onDuplicate={noop}
        onDelete={noop}
      />,
    );
    expect(screen.getByText("Ativo")).toBeDefined();
    expect(container.querySelector(".agent-card--active")).not.toBeNull();

    rerender(
      <AgentCard
        mode={mode()}
        active={false}
        onEdit={noop}
        onDuplicate={noop}
        onDelete={noop}
      />,
    );
    expect(screen.queryByText("Ativo")).toBeNull();
    expect(container.querySelector(".agent-card--active")).toBeNull();
  });

  it("aciona editar, duplicar e excluir", () => {
    const onEdit = vi.fn();
    const onDuplicate = vi.fn();
    const onDelete = vi.fn();
    render(
      <AgentCard
        mode={mode()}
        active={false}
        onEdit={onEdit}
        onDuplicate={onDuplicate}
        onDelete={onDelete}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Editar agente Vendas" }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Duplicar agente Vendas" }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Excluir agente Vendas" }),
    );

    expect(onEdit).toHaveBeenCalledTimes(1);
    expect(onDuplicate).toHaveBeenCalledTimes(1);
    expect(onDelete).toHaveBeenCalledTimes(1);
  });

  it("explica quando o agente ainda não tem prompt de resumo", () => {
    render(
      <AgentCard
        mode={mode({ summary_prompt: "" })}
        active={false}
        onEdit={noop}
        onDuplicate={noop}
        onDelete={noop}
      />,
    );
    expect(screen.getByText("Sem prompt de resumo.")).toBeDefined();
  });

  it("herda o prompt do agente Padrão quando o campo está vazio (BR-079)", () => {
    const padrao = mode({
      id: "default",
      name: "Padrão",
      suggestion_prompt: "  Contrato padrão da sugestão.  ",
      summary_prompt: "Contrato padrão do resumo.",
    });
    render(
      <AgentCard
        mode={mode({ suggestion_prompt: "   ", summary_prompt: "" })}
        defaultMode={padrao}
        active={false}
        onEdit={noop}
        onDuplicate={noop}
        onDelete={noop}
      />,
    );

    expect(screen.getByText("Contrato padrão da sugestão.")).toBeDefined();
    expect(screen.getByText("Contrato padrão do resumo.")).toBeDefined();
    expect(screen.getAllByText("Herdado do agente Padrão")).toHaveLength(2);
  });

  it("não marca como herdado quando o agente tem o próprio prompt", () => {
    const padrao = mode({ id: "default", summary_prompt: "Contrato padrão." });
    render(
      <AgentCard
        mode={mode()}
        defaultMode={padrao}
        active={false}
        onEdit={noop}
        onDuplicate={noop}
        onDelete={noop}
      />,
    );

    expect(screen.queryByText("Herdado do agente Padrão")).toBeNull();
  });
});
