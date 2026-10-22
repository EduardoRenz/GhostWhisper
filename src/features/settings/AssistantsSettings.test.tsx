import { useState } from "react";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AssistantsSettings } from "./AssistantsSettings";
import {
  defaultSettings,
  type Settings,
} from "../../core/domain/settings";
import type { AssistantMode } from "../../core/domain/assistant";

const assistantModesList = vi.fn();

vi.mock("../../adapters/ipc/suggestions", () => ({
  assistantModesList: (...args: unknown[]) => assistantModesList(...args),
}));

const agent = (overrides: Partial<AssistantMode> = {}): AssistantMode => ({
  id: "custom-a",
  name: "Vendas",
  category: "custom",
  system_prompt: "Critérios de vendas.",
  suggestion_prompt: "Sugira a próxima fala de vendas.",
  summary_prompt: "Resuma as decisões da conversa.",
  context: "",
  icon: "briefcase",
  color: "coral",
  ...overrides,
});

/** Aplica as edições no rascunho, como o `SettingsPage` faz. */
function Harness({ initial }: { initial?: Settings }) {
  const [settings, setSettings] = useState<Settings>(initial ?? defaultSettings());
  const update = <K extends keyof Settings>(field: K, value: Settings[K]) =>
    setSettings((previous) => ({ ...previous, [field]: value }));
  return <AssistantsSettings settings={settings} update={update} />;
}

function settingsWith(
  modes: AssistantMode[],
  overrides: Partial<Settings> = {},
): Settings {
  return {
    ...defaultSettings(),
    custom_assistant_modes: modes,
    assistant_modes_seeded: true,
    ...overrides,
  };
}

describe("AssistantsSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    assistantModesList.mockResolvedValue([
      {
        id: "default",
        name: "Padrão",
        category: "general",
        system_prompt: "default",
        suggestion_prompt: "default",
        summary_prompt: "default",
        context: "",
        icon: "sparkle",
        color: "slate",
      },
    ]);
  });

  it("mostra o agente Padrão no topo, protegido e sem ações", async () => {
    render(<Harness initial={settingsWith([agent()])} />);

    const padrao = await screen.findByRole("article", {
      name: "Assistente Padrão",
    });
    expect(padrao).toBeDefined();
    expect(screen.getByText("Primário")).toBeDefined();
    // Não há como editar/excluir o Padrão.
    expect(
      screen.queryByRole("button", { name: /Editar agente Padrão/ }),
    ).toBeNull();
    expect(
      screen.queryByRole("button", { name: /Excluir agente Padrão/ }),
    ).toBeNull();
  });

  it("conta os agentes configurados", () => {
    render(
      <Harness
        initial={settingsWith([
          agent(),
          agent({ id: "custom-b", name: "Suporte" }),
        ])}
      />,
    );
    expect(screen.getByText("Agentes configurados (2)")).toBeDefined();
  });

  it("marca o agente em uso com a badge Ativo", async () => {
    render(
      <Harness
        initial={settingsWith([agent()], { assistant_mode_id: "custom-a" })}
      />,
    );
    const card = await screen.findByRole("article", { name: "Agente Vendas" });
    expect(card.textContent).toContain("Ativo");
  });

  it("edita um agente pelo modal", async () => {
    render(<Harness initial={settingsWith([agent()])} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Editar agente Vendas" }),
    );

    const nome = screen.getByLabelText("Nome") as HTMLInputElement;
    expect(nome.value).toBe("Vendas");

    // Cada campo de prompt tem ajuda (BR-081). Escopo no modal: o card atrás
    // também tem os botões de ajuda dos painéis.
    const dialog = screen.getByRole("dialog", { name: "Editar agente" });
    const help = within(dialog);
    expect(
      help.getByRole("button", { name: "Ajuda: Prompt de Sugestão" }),
    ).toBeDefined();
    expect(
      help.getByRole("button", { name: "Ajuda: Prompt de Resumo" }),
    ).toBeDefined();
    fireEvent.click(help.getByText("Campos avançados"));
    expect(
      help.getByRole("button", { name: "Ajuda: Categoria" }),
    ).toBeDefined();
    expect(
      help.getByRole("button", {
        name: "Ajuda: Prompt de sistema (critérios de domínio)",
      }),
    ).toBeDefined();
    expect(
      help.getByRole("button", { name: "Ajuda: Contexto fixo do agente" }),
    ).toBeDefined();

    fireEvent.change(nome, { target: { value: "Vendas B2B" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Salvar agente" }));
    });

    expect(
      screen.getByText("Vendas B2B", { selector: ".agent-card__name" }),
    ).toBeDefined();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("cria um agente do zero pelo modal", async () => {
    render(<Harness initial={settingsWith([])} />);

    fireEvent.click(
      screen.getByRole("button", { name: /Criar Novo Assistente de IA/ }),
    );
    expect(screen.getByRole("dialog", { name: "Novo assistente de IA" })).toBeDefined();

    fireEvent.change(screen.getByLabelText("Nome"), {
      target: { value: "Suporte" },
    });
    fireEvent.change(screen.getByLabelText("Prompt de Sugestão"), {
      target: { value: "Sugira a resposta ao cliente." },
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Salvar agente" }));
    });

    expect(
      screen.getByText("Suporte", { selector: ".agent-card__name" }),
    ).toBeDefined();
    expect(screen.getByText("Agentes configurados (1)")).toBeDefined();
  });

  it("recusa um agente sem nome", async () => {
    render(<Harness initial={settingsWith([])} />);

    fireEvent.click(
      screen.getByRole("button", { name: /Criar Novo Assistente de IA/ }),
    );
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Salvar agente" }));
    });

    expect(screen.getByText("Dê um nome ao agente.")).toBeDefined();
    expect(screen.getByRole("dialog")).toBeDefined();
  });

  it("recusa um agente sem nenhum prompt", async () => {
    render(<Harness initial={settingsWith([])} />);

    fireEvent.click(
      screen.getByRole("button", { name: /Criar Novo Assistente de IA/ }),
    );
    fireEvent.change(screen.getByLabelText("Nome"), {
      target: { value: "Vazio" },
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Salvar agente" }));
    });

    expect(
      screen.getByText("Preencha ao menos um prompt ou o contexto."),
    ).toBeDefined();
  });

  it("duplica um agente com o nome de cópia", async () => {
    render(<Harness initial={settingsWith([agent()])} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Duplicar agente Vendas" }),
    );

    const nome = screen.getByLabelText("Nome") as HTMLInputElement;
    expect(nome.value).toBe("Vendas (cópia)");
    expect(screen.getByRole("dialog", { name: "Novo assistente de IA" })).toBeDefined();
  });

  it("exclui um agente", async () => {
    render(<Harness initial={settingsWith([agent()])} />);

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "Excluir agente Vendas" }),
      );
    });

    expect(screen.queryByRole("article", { name: "Agente Vendas" })).toBeNull();
    expect(screen.getByText("Agentes configurados (0)")).toBeDefined();
  });

  it("excluir o agente ativo volta a seleção para o Padrão", async () => {
    render(
      <Harness
        initial={settingsWith([agent()], { assistant_mode_id: "custom-a" })}
      />,
    );

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "Excluir agente Vendas" }),
      );
    });

    const padrao = screen.getByRole("article", { name: "Assistente Padrão" });
    expect(padrao.textContent).toContain("Ativo");
  });

  it("usa o fallback do nome do Padrão fora do catálogo", async () => {
    assistantModesList.mockResolvedValue([]);
    render(<Harness initial={settingsWith([])} />);

    await waitFor(() =>
      expect(
        screen.getByRole("article", { name: "Assistente Padrão" }).textContent,
      ).toContain("Padrão"),
    );
  });
});
