import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AssistantSettings } from "./AssistantSettings";
import { defaultSettings, type Settings } from "../../core/domain/settings";
import type { AssistantMode } from "../../core/domain/assistant";

const assistantModesList = vi.fn();

vi.mock("../../adapters/ipc/suggestions", () => ({
  assistantModesList: (...args: unknown[]) => assistantModesList(...args),
}));

const builtin = (id: string, name: string): AssistantMode => ({
  id,
  name,
  category: "engineering",
  system_prompt: "system",
  suggestion_prompt: "suggestion",
  context: "",
  icon: "sparkle",
  color: "slate",
});

describe("AssistantSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    assistantModesList.mockResolvedValue([
      builtin("meeting", "Reunião"),
      builtin("leetcode", "LeetCode"),
    ]);
  });

  it("salva um agente personalizado válido", async () => {
    const update = vi.fn();
    render(
      <AssistantSettings settings={defaultSettings()} update={update} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Novo agente" }));
    fireEvent.change(screen.getByLabelText("Nome"), {
      target: { value: "Vendas" },
    });
    fireEvent.change(
      screen.getByLabelText("Prompt de sugestão ao vivo"),
      { target: { value: "Sugira a próxima fala de vendas." } },
    );
    fireEvent.click(screen.getByRole("button", { name: "Salvar agente" }));

    expect(update).toHaveBeenCalledWith(
      "custom_assistant_modes",
      expect.arrayContaining([
        expect.objectContaining({
          name: "Vendas",
          suggestion_prompt: "Sugira a próxima fala de vendas.",
        }),
      ]),
    );
  });

  it("recusa um agente sem nome", () => {
    const update = vi.fn();
    render(
      <AssistantSettings settings={defaultSettings()} update={update} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Novo agente" }));
    fireEvent.click(screen.getByRole("button", { name: "Salvar agente" }));

    expect(update).not.toHaveBeenCalled();
    expect(screen.getByText("Dê um nome ao agente.")).toBeDefined();
  });

  it("duplica um agente embutido com prompts e novo id", async () => {
    const update = vi.fn();
    render(
      <AssistantSettings settings={defaultSettings()} update={update} />,
    );

    const duplicate = await screen.findByRole("button", {
      name: "Duplicar LeetCode",
    });
    fireEvent.click(duplicate);

    expect(await screen.findByDisplayValue("LeetCode (cópia)")).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Salvar agente" }));

    const saved = update.mock.calls.find(
      ([field]) => field === "custom_assistant_modes",
    )?.[1] as AssistantMode[] | undefined;
    expect(saved?.[0]?.suggestion_prompt).toBe("suggestion");
    expect(saved?.[0]?.id.startsWith("custom-")).toBe(true);
  });

  it("exclui um agente e volta a seleção para o agente Padrão", () => {
    const update = vi.fn();
    const settings: Settings = {
      ...defaultSettings(),
      assistant_mode_id: "custom-vendas",
      custom_assistant_modes: [builtin("custom-vendas", "Vendas")],
    };
    render(<AssistantSettings settings={settings} update={update} />);

    fireEvent.click(screen.getByRole("button", { name: "Excluir" }));

    expect(update).toHaveBeenCalledWith("custom_assistant_modes", []);
    expect(update).toHaveBeenCalledWith("assistant_mode_id", "default");
  });
});
