import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SuggestionModelDownloadEvent } from "../domain/classifierModel";

const ipc = vi.hoisted(() => ({
  backend: vi.fn(),
  catalog: vi.fn(),
  installed: vi.fn(),
  active: vi.fn(),
  install: vi.fn(),
  activate: vi.fn(),
  remove: vi.fn(),
}));

const events = vi.hoisted(() => ({
  handler: null as ((event: SuggestionModelDownloadEvent) => void) | null,
  subscribe: vi.fn(),
}));

vi.mock("../../adapters/ipc/classifierModels", () => ({
  classifierBackendStatus: ipc.backend,
  classifierModelCatalog: ipc.catalog,
  classifierModelInstalled: ipc.installed,
  classifierModelActive: ipc.active,
  classifierModelInstall: ipc.install,
  classifierModelActivate: ipc.activate,
  classifierModelRemove: ipc.remove,
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: (
    _event: string,
    handler: (event: SuggestionModelDownloadEvent) => void,
  ) => {
    events.handler = handler;
    events.subscribe();
    return Promise.resolve(() => {});
  },
}));

import { useClassifierModels } from "./useClassifierModels";

function Harness() {
  const models = useClassifierModels();
  return (
    <div>
      <span data-testid="engine">
        {models.backend?.laya_feature ? "on" : "off"}
      </span>
      <span data-testid="active">{models.active?.version ?? "none"}</span>
      <span data-testid="installed">{models.installed.length}</span>
      <span data-testid="phase">{models.progress?.phase ?? "idle"}</span>
      <span data-testid="error">{models.error ?? ""}</span>
      <button
        type="button"
        onClick={() => void models.install("laya-multilingual", "0.1.0")}
      >
        baixar
      </button>
      <button type="button" onClick={() => void models.refresh()}>
        atualizar
      </button>
    </div>
  );
}

describe("useClassifierModels", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    ipc.backend.mockResolvedValue({
      laya_feature: true,
      model_active: false,
      model_configured: false,
      model_bundled: false,
    });
    ipc.catalog.mockResolvedValue({
      schema: 1,
      models: [
        {
          schema: 1,
          model_id: "laya-multilingual",
          version: "0.1.0",
          checkpoint: "multilingual",
          layout: "onnx",
          size_bytes: 10,
          license: "Apache-2.0",
          files: [],
          calibration: {},
          metrics: {},
          created_at: "2026-10-26",
        },
      ],
    });
    ipc.installed.mockResolvedValue([]);
    ipc.active.mockResolvedValue(null);
    ipc.install.mockResolvedValue(null);
    events.handler = null;
    events.subscribe.mockClear();
  });

  it("hidrata catálogo, instaladas e ativa", async () => {
    ipc.installed.mockResolvedValue([
      {
        model_id: "laya-multilingual",
        version: "0.1.0",
        checkpoint: "multilingual",
        path: "/tmp/laya",
        size_bytes: 10,
        active: true,
        verified: true,
        installed_at_ms: 1,
      },
    ]);
    ipc.active.mockResolvedValue({
      model_id: "laya-multilingual",
      version: "0.1.0",
      checkpoint: "multilingual",
      path: "/tmp/laya",
      size_bytes: 10,
      active: true,
      verified: true,
      installed_at_ms: 1,
    });

    render(<Harness />);

    await waitFor(() => expect(screen.getByTestId("active").textContent).toBe("0.1.0"));
    expect(screen.getByTestId("installed").textContent).toBe("1");
    expect(screen.getByTestId("engine").textContent).toBe("on");
    expect(events.subscribe).toHaveBeenCalledOnce();
  });

  it("instala via IPC e atualiza o progresso pelo evento", async () => {
    render(<Harness />);
    await waitFor(() => expect(events.handler).not.toBeNull());

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "baixar" }));
    });

    expect(ipc.install).toHaveBeenCalledWith("laya-multilingual", "0.1.0");
    expect(screen.getByTestId("phase").textContent).toBe("downloading");

    act(() => {
      events.handler?.({
        model_id: "laya-multilingual",
        version: "0.1.0",
        phase: "ready",
        file: null,
        file_index: 3,
        file_count: 3,
        bytes_done: 10,
        bytes_total: 10,
        error: null,
      });
    });

    await waitFor(() => expect(screen.getByTestId("phase").textContent).toBe("ready"));
    // `ready` recarrega o estado do core.
    expect(ipc.installed.mock.calls.length).toBeGreaterThanOrEqual(2);
  });

  it("sem a feature laya não bate no catálogo (só lista o disco)", async () => {
    ipc.backend.mockResolvedValue({
      laya_feature: false,
      model_active: false,
      model_configured: false,
      model_bundled: false,
    });
    render(<Harness />);

    await waitFor(() => expect(screen.getByTestId("engine").textContent).toBe("off"));
    expect(ipc.catalog).not.toHaveBeenCalled();
    expect(screen.getByTestId("error").textContent).toBe("");
  });

  it("com modelo empacotado, catálogo offline não vira erro", async () => {
    ipc.backend.mockResolvedValue({
      laya_feature: true,
      model_active: false,
      model_configured: true,
      model_bundled: true,
    });
    ipc.catalog.mockRejectedValue(new Error("offline"));
    render(<Harness />);

    await waitFor(() => expect(screen.getByTestId("engine").textContent).toBe("on"));
    expect(screen.getByTestId("error").textContent).toBe("");
  });

  it("expõe erro do IPC sem quebrar a tela", async () => {
    ipc.catalog.mockRejectedValue(new Error("offline"));
    render(<Harness />);

    await waitFor(() => expect(screen.getByTestId("error").textContent).toContain("offline"));
  });
});
