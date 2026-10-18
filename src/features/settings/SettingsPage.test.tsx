import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsPage } from "./SettingsPage";

const settingsIpc = vi.hoisted(() => ({
  clearProviderKey: vi.fn(async () => undefined),
  getProviders: vi.fn(async () => ["openai", "gemini", "openrouter", "custom"]),
  getSettings: vi.fn(async () => undefined),
  gpuInfo: vi.fn(async () => undefined),
  hasProviderKey: vi.fn(async () => false),
  keyPreview: vi.fn(async () => ""),
  providerLabel: vi.fn(async () => "OpenAI · gpt-4o-mini"),
  saveSettings: vi.fn(async () => []),
  secretsPersistent: vi.fn(async () => true),
  setProviderKey: vi.fn(async () => undefined),
  validateKey: vi.fn(async () => "ok"),
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
vi.mock("../../adapters/ipc/shortcuts", () => shortcutsIpc);
vi.mock("../../adapters/ipc/audio", () => audioIpc);

describe("SettingsPage", () => {
  beforeEach(() => {
    settingsIpc.hasProviderKey.mockResolvedValue(false);
    settingsIpc.keyPreview.mockResolvedValue("");
    settingsIpc.gpuInfo.mockResolvedValue(undefined);
  });

  it("renderiza as abas e mostra a seção de IA por padrão", async () => {
    render(<SettingsPage />);

    expect(await screen.findByRole("tab", { name: "Fontes de IA" })).toBeDefined();
    expect(screen.getByRole("tablist").children.length).toBe(5);
    expect(screen.getByRole("tab", { name: "Assistente" })).toBeDefined();
    expect(await screen.findByText("Provider")).toBeDefined();
  });

  it("mostra a chave mascarada quando já configurada", async () => {
    settingsIpc.hasProviderKey.mockResolvedValue(true);
    settingsIpc.keyPreview.mockResolvedValue("••••••••·1234");

    render(<SettingsPage />);

    const masked = await screen.findByLabelText("Chave de API (mascarada)");
    expect((masked as HTMLInputElement).value).toBe("••••••••·1234");
    // Sem campo de digitação/salvar enquanto a chave existe.
    expect(screen.queryByPlaceholderText("sk-...")).toBeNull();
    expect(screen.queryByRole("button", { name: "Salvar chave" })).toBeNull();
  });

  it("pede uma nova chave quando nenhuma está configurada", async () => {
    render(<SettingsPage />);

    expect(await screen.findByPlaceholderText("sk-...")).toBeDefined();
    expect(
      screen.queryByLabelText("Chave de API (mascarada)"),
    ).toBeNull();
  });

  it("troca para a aba de transcrição e mostra o modelo whisper", async () => {
    render(<SettingsPage />);

    await act(async () => {
      screen.getByRole("tab", { name: "Transcrição & Áudio" }).click();
    });

    expect(await screen.findByText("Modelo whisper (local)")).toBeDefined();
    expect(screen.queryByText("Provider")).toBeNull();
  });

  it("expõe a sensibilidade do microfone com o padrão do Perssua", async () => {
    render(<SettingsPage />);

    await act(async () => {
      screen.getByRole("tab", { name: "Transcrição & Áudio" }).click();
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
      screen.getByRole("tab", { name: "Transcrição & Áudio" }).click();
    });

    const hint = await screen.findByTestId("gpu-hint");
    expect(hint.textContent).toContain("AMD (RADV");
    expect(hint.textContent).toContain("large-v3-turbo");
    // Nada é trocado automaticamente: o default continua tiny.
    expect(
      (screen.getByLabelText("Modelo whisper (local)") as HTMLSelectElement).value,
    ).toBe("tiny");
  });
});
