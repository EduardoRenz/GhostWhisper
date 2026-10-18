import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AudioTestPanel } from "./AudioTestPanel";

const audioIpc = vi.hoisted(() => ({
  listMicrophones: vi.fn(async () => [
    { id: "mic-1", name: "Mic interno", is_default: true },
  ]),
  listSystemSources: vi.fn(async () => [
    { id: "monitor-1", name: "Monitor", is_default: true },
  ]),
  monitorDefaultDelay: vi.fn(async () => 400),
  startMicrophone: vi.fn(async () => undefined),
  startMonitor: vi.fn(async () => undefined),
  startSystemAudio: vi.fn(async () => undefined),
  stopMicrophone: vi.fn(async () => undefined),
  stopMonitor: vi.fn(async () => undefined),
  stopSystemAudio: vi.fn(async () => undefined),
}));

vi.mock("../../adapters/ipc/audio", () => audioIpc);

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: async () => () => {},
}));

describe("AudioTestPanel — monitor atrasado", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    audioIpc.monitorDefaultDelay.mockResolvedValue(400);
  });

  it("mostra o toggle só no microfone e desabilitado antes do teste", async () => {
    render(<AudioTestPanel />);

    const toggle = await screen.findByRole("checkbox", { name: /Ouvir minha voz/ });
    expect(toggle).toHaveProperty("disabled", true);
    // O atraso vem do core e aparece no rótulo.
    await waitFor(() => {
      expect(screen.getByText(/0\.4 s de atraso/)).toBeDefined();
    });

    // No áudio do sistema não faz sentido monitorar o próprio microfone.
    fireEvent.change(screen.getByRole("combobox", { name: /Canal/ }), {
      target: { value: "system" },
    });
    expect(screen.queryByRole("checkbox", { name: /Ouvir minha voz/ })).toBeNull();
  });

  it("liga e desliga o monitor depois de iniciar o teste", async () => {
    render(<AudioTestPanel />);
    await screen.findByRole("checkbox", { name: /Ouvir minha voz/ });

    fireEvent.click(screen.getByRole("button", { name: "Iniciar teste" }));
    await waitFor(() => {
      expect(audioIpc.startMicrophone).toHaveBeenCalled();
    });

    const toggle = screen.getByRole("checkbox", { name: /Ouvir minha voz/ });
    expect(toggle).toHaveProperty("disabled", false);

    fireEvent.click(toggle);
    await waitFor(() => {
      expect(audioIpc.startMonitor).toHaveBeenCalledWith(400);
    });

    fireEvent.click(toggle);
    await waitFor(() => {
      expect(audioIpc.stopMonitor).toHaveBeenCalled();
    });
  });

  it("desliga o monitor ao parar o teste", async () => {
    render(<AudioTestPanel />);
    await screen.findByRole("checkbox", { name: /Ouvir minha voz/ });

    fireEvent.click(screen.getByRole("button", { name: "Iniciar teste" }));
    await waitFor(() => {
      expect(audioIpc.startMicrophone).toHaveBeenCalled();
    });
    fireEvent.click(screen.getByRole("checkbox", { name: /Ouvir minha voz/ }));
    await waitFor(() => {
      expect(audioIpc.startMonitor).toHaveBeenCalled();
    });

    fireEvent.click(screen.getByRole("button", { name: "Parar teste" }));

    await waitFor(() => {
      expect(audioIpc.stopMonitor).toHaveBeenCalled();
    });
    expect(audioIpc.stopMicrophone).toHaveBeenCalled();
  });
});
