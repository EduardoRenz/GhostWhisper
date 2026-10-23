import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsPage } from "./SettingsPage";
import { defaultSettings } from "../../core/domain/settings";
import type { AssistantMode } from "../../core/domain/assistant";

const eventsIpc = vi.hoisted(() => ({
  subscribe: vi.fn(
    async (_event?: string, _handler?: unknown) => () => {},
  ),
}));

const settingsIpc = vi.hoisted(() => ({
  closeSettings: vi.fn(async () => undefined),
  getProviders: vi.fn(async () => [
    "openai",
    "gemini",
    "openrouter",
    "deepseek",
    "custom",
  ]),
  getSettings: vi.fn(async () => undefined),
  gpuInfo: vi.fn(async () => undefined),
  previewFontScales: vi.fn(async () => undefined),
  previewOverlayOpacity: vi.fn(async () => undefined),
  saveSettings: vi.fn(async (_settings: unknown) => [] as unknown[]),
  setFontScales: vi.fn(async () => undefined),
  setOverlayOpacity: vi.fn(async () => undefined),
  secretsPersistent: vi.fn(async () => true),
  settingsDefaults: vi.fn(async () => undefined),
  validateKey: vi.fn(async () => "ok"),
  providerLabel: vi.fn(async () => "OpenAI · gpt-4o-mini"),
}));

const suggestionsIpc = vi.hoisted(() => ({
  assistantModesList: vi.fn(async () => []),
}));

const aiSourcesIpc = vi.hoisted(() => ({
  clearSourceKey: vi.fn(async () => undefined),
  hasSourceKey: vi.fn(async () => false),
  listSources: vi.fn(async () => []),
  setSourceKey: vi.fn(async () => undefined),
  sourceKeyPreview: vi.fn(async () => ""),
  sourcesHealth: vi.fn(async () => []),
  validateSource: vi.fn(async () => "ok"),
}));

const shortcutsIpc = vi.hoisted(() => ({
  getShortcutCatalog: vi.fn(async () => []),
  setShortcuts: vi.fn(async () => []),
}));

const panelsIpc = vi.hoisted(() => ({
  panelStartDrag: vi.fn(),
}));

const audioIpc = vi.hoisted(() => ({
  AudioDevice: {},
  listMicrophones: vi.fn(async () => []),
  listSystemSources: vi.fn(async () => []),
  monitorDefaultDelay: vi.fn(async () => 400),
  monitorIsRunning: vi.fn(async () => false),
  startMicrophone: vi.fn(async () => undefined),
  startMonitor: vi.fn(async () => undefined),
  startSystemAudio: vi.fn(async () => undefined),
  stopMicrophone: vi.fn(async () => undefined),
  stopMonitor: vi.fn(async () => undefined),
  stopSystemAudio: vi.fn(async () => undefined),
}));

vi.mock("../../adapters/ipc/settings", () => settingsIpc);
vi.mock("../../adapters/ipc/aiSources", () => aiSourcesIpc);
vi.mock("../../adapters/ipc/shortcuts", () => shortcutsIpc);
vi.mock("../../adapters/ipc/audio", () => audioIpc);
vi.mock("../../adapters/ipc/panels", () => panelsIpc);
vi.mock("../../adapters/ipc/suggestions", () => suggestionsIpc);
vi.mock("../../adapters/ipc/events", () => eventsIpc);

describe("SettingsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    settingsIpc.getSettings.mockResolvedValue(undefined);
    settingsIpc.saveSettings.mockResolvedValue([]);
    settingsIpc.getProviders.mockResolvedValue([
      "openai",
      "gemini",
      "openrouter",
      "deepseek",
      "custom",
    ]);
    settingsIpc.secretsPersistent.mockResolvedValue(true);
    settingsIpc.gpuInfo.mockResolvedValue(undefined);
    aiSourcesIpc.hasSourceKey.mockResolvedValue(false);
    aiSourcesIpc.sourceKeyPreview.mockResolvedValue("");
    aiSourcesIpc.sourcesHealth.mockResolvedValue([]);
  });

  it("mostra as 6 abas com ícone e rótulo", async () => {
    render(<SettingsPage />);

    const tabs = await screen.findAllByRole("tab");
    expect(tabs.length).toBe(6);
    expect(screen.getByRole("tab", { name: /Fontes de IA/ })).toBeDefined();
    expect(screen.getByRole("tab", { name: /Transcrição & Áudio/ })).toBeDefined();
    expect(screen.getByRole("tab", { name: /Gravação/ })).toBeDefined();
    expect(screen.getByRole("tab", { name: /Assistentes/ })).toBeDefined();
    expect(screen.getByRole("tab", { name: /Geral & HUD/ })).toBeDefined();
    expect(screen.getByRole("tab", { name: /Atalhos/ })).toBeDefined();
  });

  it("abre na aba Geral & HUD (primeira) com o idioma da interface", async () => {
    render(<SettingsPage />);

    const tabs = await screen.findAllByRole("tab");
    expect(tabs[0]?.getAttribute("aria-selected")).toBe("true");
    expect(tabs[0]?.textContent).toContain("Geral & HUD");
    expect(await screen.findByLabelText("Idioma da interface")).toBeDefined();
  });

  it("tem cabeçalho e rodapé com salvamento global", async () => {
    render(<SettingsPage />);

    expect(await screen.findByText("Configurações")).toBeDefined();
    expect(
      screen.getByText(/Personalize captura de áudio, IA local/),
    ).toBeDefined();
    expect(screen.getByRole("button", { name: "Salvar Alterações" })).toBeDefined();
    expect(
      screen.getByRole("button", { name: "Restaurar Padrões" }),
    ).toBeDefined();
    // Não há mais botão de salvar dentro das abas.
    expect(
      screen.queryByRole("button", { name: "Salvar configurações" }),
    ).toBeNull();
  });

  it("fecha a janela pelo botão ✕", async () => {
    render(<SettingsPage />);

    const close = await screen.findByRole("button", {
      name: "Fechar configurações",
    });
    fireEvent.click(close);
    await waitFor(() => expect(settingsIpc.closeSettings).toHaveBeenCalled());
  });

  it("arrasta pelo grip no canto superior esquerdo", async () => {
    render(<SettingsPage />);

    const drag = await screen.findByLabelText("Mover configurações");
    fireEvent.pointerDown(drag);
    expect(panelsIpc.panelStartDrag).toHaveBeenCalled();
  });

  it("salva o rascunho inteiro uma única vez", async () => {
    render(<SettingsPage />);

    const save = await screen.findByRole("button", { name: "Salvar Alterações" });
    await act(async () => {
      fireEvent.click(save);
    });

    await waitFor(() => expect(settingsIpc.saveSettings).toHaveBeenCalledTimes(1));
    const saved = settingsIpc.saveSettings.mock.calls[0]?.[0] as unknown as {
      ai_sources: unknown[];
    };
    expect(Array.isArray(saved.ai_sources)).toBe(true);
    expect(screen.getByText("Configurações salvas.")).toBeDefined();
  });

  it("pede confirmação antes de restaurar os padrões", async () => {
    const confirmSpy = vi
      .spyOn(window, "confirm")
      .mockReturnValue(false);
    render(<SettingsPage />);

    const restore = await screen.findByRole("button", {
      name: "Restaurar Padrões",
    });
    await act(async () => {
      fireEvent.click(restore);
    });

    expect(confirmSpy).toHaveBeenCalled();
    // Cancelou: nada foi salvo.
    expect(settingsIpc.saveSettings).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });

  it("restaura os padrões do core, com os agentes de fábrica", async () => {
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    settingsIpc.settingsDefaults.mockResolvedValue({
      ...defaultSettings(),
      custom_assistant_modes: [
        {
          id: "meeting",
          name: "Reunião",
          category: "productivity",
          system_prompt: "s",
          suggestion_prompt: "s",
          summary_prompt: "r",
          context: "",
          icon: "users",
          color: "cyan",
        },
      ],
      assistant_modes_seeded: true,
    } as never);

    render(<SettingsPage />);
    const restore = await screen.findByRole("button", {
      name: "Restaurar Padrões",
    });
    await act(async () => {
      fireEvent.click(restore);
    });

    await waitFor(() => expect(settingsIpc.settingsDefaults).toHaveBeenCalled());
    expect(
      screen.getByText(/Padrões restaurados/),
    ).toBeDefined();
    confirmSpy.mockRestore();
  });

  it("mostra o painel de Assistentes com o agente Padrão", async () => {
    render(<SettingsPage />);

    await act(async () => {
      screen.getByRole("tab", { name: /Assistentes/ }).click();
    });

    expect(
      await screen.findByRole("article", { name: "Assistente Padrão" }),
    ).toBeDefined();
    expect(
      screen.getByRole("button", { name: /Criar Novo Assistente de IA/ }),
    ).toBeDefined();
    expect(screen.getByText("Agentes configurados (0)")).toBeDefined();
  });

  it("mostra cadências e perguntas prontas na aba Geral & HUD", async () => {
    render(<SettingsPage />);

    await act(async () => {
      screen.getByRole("tab", { name: /Geral & HUD/ }).click();
    });

    expect(await screen.findByText("Cadências & Perguntas Prontas")).toBeDefined();
    expect(screen.getByText("Perguntas prontas")).toBeDefined();
  });

  it("sincroniza o agente ativo trocado em outra janela (BR-080)", async () => {
    const leetcode: AssistantMode = {
      id: "leetcode",
      name: "LeetCode",
      category: "engineering",
      system_prompt: "s",
      suggestion_prompt: "s",
      summary_prompt: "r",
      context: "",
      icon: "code",
      color: "emerald",
    };
    const base = {
      ...defaultSettings(),
      custom_assistant_modes: [leetcode],
    };
    settingsIpc.getSettings.mockResolvedValue(base as never);

    let handler: (() => void) | undefined;
    eventsIpc.subscribe.mockImplementation(
      async (_event?: string, callback?: unknown) => {
        handler = callback as (() => void) | undefined;
        return () => {};
      },
    );

    render(<SettingsPage />);
    await waitFor(() => expect(handler).toBeDefined());

    // O seletor global do notch troca o agente em outra janela.
    settingsIpc.getSettings.mockResolvedValue({
      ...base,
      assistant_mode_id: "leetcode",
    } as never);
    await act(async () => {
      handler?.();
    });

    await act(async () => {
      screen.getByRole("tab", { name: /Assistentes/ }).click();
    });
    const card = await screen.findByRole("article", { name: "Agente LeetCode" });
    expect(card.textContent).toContain("Ativo");
  });

  it("aplica a opacidade em tempo real e salva automaticamente", async () => {
    render(<SettingsPage />);

    await act(async () => {
      screen.getByRole("tab", { name: /Geral & HUD/ }).click();
    });

    const slider = await screen.findByLabelText("Opacidade das janelas");
    await act(async () => {
      fireEvent.change(slider, { target: { value: "40" } });
    });

    expect(settingsIpc.previewOverlayOpacity).toHaveBeenCalledWith(0.4);
    expect(
      document.documentElement.style.getPropertyValue("--hud-alpha"),
    ).toBe("0.4");
    await waitFor(
      () => expect(settingsIpc.setOverlayOpacity).toHaveBeenCalledWith(0.4),
      { timeout: 1000 },
    );
    // Não usa o save global: o autosave é só da opacidade.
    expect(settingsIpc.saveSettings).not.toHaveBeenCalled();
  });

  it("aplica a escala de fonte em tempo real e salva automaticamente", async () => {
    document.documentElement.style.removeProperty("font-size");
    document.documentElement.style.removeProperty("--oh-ai-font-scale");
    render(<SettingsPage />);

    await act(async () => {
      screen.getByRole("tab", { name: /Geral & HUD/ }).click();
    });

    const fontSlider = await screen.findByLabelText("Escala da fonte");
    await act(async () => {
      fireEvent.change(fontSlider, { target: { value: "140" } });
    });

    expect(settingsIpc.previewFontScales).toHaveBeenCalledWith(1.4, 1);
    // O jsdom normaliza `calc(16px * 1.4)` para `calc(22.4px)`; o navegador
    // preserva a forma autorada. O teste aceita as duas.
    expect(
      [
        "calc(16px * 1.4)",
        `calc(${Number((16 * 1.4).toFixed(4))}px)`,
      ].includes(document.documentElement.style.getPropertyValue("font-size")),
    ).toBe(true);

    const aiSlider = await screen.findByLabelText("Escala do texto de IA");
    await act(async () => {
      fireEvent.change(aiSlider, { target: { value: "130" } });
    });

    expect(settingsIpc.previewFontScales).toHaveBeenCalledWith(1.4, 1.3);
    expect(
      document.documentElement.style.getPropertyValue("--oh-ai-font-scale"),
    ).toBe("1.3");

    await waitFor(
      () => expect(settingsIpc.setFontScales).toHaveBeenCalledWith(1.4, 1.3),
      { timeout: 1000 },
    );
    // Não usa o save global: o autosave é só das escalas de fonte.
    expect(settingsIpc.saveSettings).not.toHaveBeenCalled();

    document.documentElement.style.removeProperty("font-size");
    document.documentElement.style.removeProperty("--oh-ai-font-scale");
  });

  it("troca para a aba de transcrição e mostra o modelo whisper", async () => {
    render(<SettingsPage />);

    await act(async () => {
      screen.getByRole("tab", { name: /Transcrição & Áudio/ }).click();
    });

    expect(await screen.findByText("Modelo whisper (local)")).toBeDefined();
    // Seletor de aceleração presente (padrão "auto").
    expect(
      (screen.getByLabelText("Aceleração") as HTMLSelectElement).value,
    ).toBe("auto");
  });

  it("expõe a sensibilidade do microfone com o padrão do Perssua", async () => {
    render(<SettingsPage />);

    await act(async () => {
      screen.getByRole("tab", { name: /Transcrição & Áudio/ }).click();
    });

    const slider = (await screen.findByLabelText(
      "Sensibilidade do microfone",
    )) as HTMLInputElement;
    expect(slider.type).toBe("range");
    expect(slider.value).toBe("8");
    expect(slider.min).toBe("1");
    expect(slider.max).toBe("12");
    expect(screen.getByText("8.0× (padrão)")).toBeDefined();

    fireEvent.change(slider, { target: { value: "12" } });
    expect(screen.getByText("12.0×")).toBeDefined();
    expect(slider.value).toBe("12");
  });

  it("sugere o modelo quando detecta GPU (sem trocar sozinho)", async () => {
    settingsIpc.gpuInfo.mockResolvedValue({
      available: true,
      backend: "vulkan",
      detail: "AMD (RADV · radeon_icd.x86_64.json)",
      acceleration_enabled: true,
      recommended_model: "large-v3-turbo",
    } as never);

    render(<SettingsPage />);
    await act(async () => {
      screen.getByRole("tab", { name: /Transcrição & Áudio/ }).click();
    });

    const hint = await screen.findByTestId("gpu-hint");
    expect(hint.textContent).toContain("AMD (RADV");
    expect(hint.textContent).toContain("large-v3-turbo");
    // Nada é trocado automaticamente: o default continua tiny.
    expect(
      (screen.getByLabelText("Modelo whisper (local)") as HTMLSelectElement).value,
    ).toBe("tiny");
  });

  it("configura o drill-hover na aba Geral & HUD", async () => {
    render(<SettingsPage />);

    await act(async () => {
      screen.getByRole("tab", { name: /Geral & HUD/ }).click();
    });

    const toggle = (await screen.findByRole("checkbox", {
      name: "Expandir notch por dwell",
    })) as HTMLInputElement;
    expect(toggle.checked).toBe(true);

    const slider = screen.getByLabelText(
      "Tempo de dwell em milissegundos",
    ) as HTMLInputElement;
    expect(slider.value).toBe("500");
    expect(slider.min).toBe("100");
    expect(slider.max).toBe("1500");
    expect(slider.disabled).toBe(false);

    fireEvent.change(slider, { target: { value: "1200" } });
    expect(screen.getByText("1200ms")).toBeDefined();

    fireEvent.click(toggle);
    expect(toggle.checked).toBe(false);
    expect(slider.disabled).toBe(true);
  });

  it("marca as alterações não salvas quando algo muda", async () => {
    render(<SettingsPage />);

    await act(async () => {
      screen.getByRole("tab", { name: /Geral & HUD/ }).click();
    });
    const toggle = (await screen.findByRole("checkbox", {
      name: "Expandir notch por dwell",
    })) as HTMLInputElement;

    expect(screen.getByText(/sincronizadas localmente/)).toBeDefined();
    fireEvent.click(toggle);
    expect(await screen.findByText("Alterações não salvas")).toBeDefined();
  });
});
