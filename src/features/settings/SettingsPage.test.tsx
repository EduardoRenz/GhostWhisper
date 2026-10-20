import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsPage } from "./SettingsPage";

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
  saveSettings: vi.fn(async (_settings: unknown) => [] as unknown[]),
  secretsPersistent: vi.fn(async () => true),
  validateKey: vi.fn(async () => "ok"),
  providerLabel: vi.fn(async () => "OpenAI · gpt-4o-mini"),
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

  it("mostra as 5 abas com ícone e rótulo", async () => {
    render(<SettingsPage />);

    const tabs = await screen.findAllByRole("tab");
    expect(tabs.length).toBe(5);
    expect(screen.getByRole("tab", { name: /Fontes de IA/ })).toBeDefined();
    expect(screen.getByRole("tab", { name: /Transcrição & Áudio/ })).toBeDefined();
    expect(screen.getByRole("tab", { name: /Assistente/ })).toBeDefined();
    expect(screen.getByRole("tab", { name: /Geral & HUD/ })).toBeDefined();
    expect(screen.getByRole("tab", { name: /Atalhos/ })).toBeDefined();
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
