import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsPage } from "./SettingsPage";

const settingsIpc = vi.hoisted(() => ({
  clearProviderKey: vi.fn(async () => undefined),
  getProviders: vi.fn(async () => ["openai", "gemini", "openrouter", "custom"]),
  getSettings: vi.fn(async () => undefined),
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
  startMicrophone: vi.fn(async () => undefined),
  startSystemAudio: vi.fn(async () => undefined),
  stopMicrophone: vi.fn(async () => undefined),
  stopSystemAudio: vi.fn(async () => undefined),
}));

vi.mock("../../adapters/ipc/settings", () => settingsIpc);
vi.mock("../../adapters/ipc/shortcuts", () => shortcutsIpc);
vi.mock("../../adapters/ipc/audio", () => audioIpc);

describe("SettingsPage", () => {
  beforeEach(() => {
    settingsIpc.hasProviderKey.mockResolvedValue(false);
    settingsIpc.keyPreview.mockResolvedValue("");
  });

  it("renderiza as abas e mostra a seção de IA por padrão", async () => {
    render(<SettingsPage />);

    expect(await screen.findByRole("tab", { name: "Fontes de IA" })).toBeDefined();
    expect(screen.getByRole("tablist").children.length).toBe(4);
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
});
