import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type {
  ClassifierModelCatalog,
  InstalledClassifierModel,
} from "../../core/domain/classifierModel";
import type { ClassifierModelsView } from "../../core/application/useClassifierModels";
import { ClassifierModelManager } from "./ClassifierModelManager";

const state = vi.hoisted(() => ({ value: {} as ClassifierModelsView }));

vi.mock(
  "../../core/application/useClassifierModels",
  async (importOriginal) => {
    const actual =
      await importOriginal<
        typeof import("../../core/application/useClassifierModels")
      >();
    return { ...actual, useClassifierModels: () => state.value };
  },
);

const catalog: ClassifierModelCatalog = {
  schema: 1,
  models: [
    {
      schema: 1,
      model_id: "laya-multilingual",
      version: "0.1.0",
      checkpoint: "multilingual",
      layout: "onnx",
      label: "Laya Multilingual",
      size_bytes: 1_200_000_000,
      license: "Apache-2.0",
      files: [],
      calibration: {},
      metrics: {},
      created_at: "2026-10-26",
    },
  ],
};

const installed = (
  overrides: Partial<InstalledClassifierModel> = {},
): InstalledClassifierModel => ({
  model_id: "laya-multilingual",
  version: "0.1.0",
  checkpoint: "multilingual",
  path: "/tmp/laya",
  size_bytes: 1_200_000_000,
  active: false,
  verified: true,
  installed_at_ms: 1,
  ...overrides,
});

function view(overrides: Partial<ClassifierModelsView> = {}): ClassifierModelsView {
  return {
    catalog,
    backend: {
      laya_feature: true,
      model_active: false,
      model_configured: false,
      model_bundled: false,
    },
    recommended: catalog.models[0] ?? null,
    installed: [],
    active: null,
    loading: false,
    busy: false,
    error: null,
    progress: null,
    refresh: vi.fn(async () => {}),
    install: vi.fn(async () => {}),
    activate: vi.fn(async () => {}),
    remove: vi.fn(async () => {}),
    ...overrides,
  };
}

function renderManager(overrides: Partial<ClassifierModelsView> = {}) {
  state.value = view(overrides);
  render(<ClassifierModelManager modelId="laya-multilingual" checkpoint="multilingual" />);
  return state.value;
}

describe("ClassifierModelManager", () => {
  it("mostra o recomendado e exige confirmação antes de baixar", () => {
    const models = renderManager();
    expect(screen.getByText("Nenhum modelo instalado")).toBeDefined();
    expect(screen.getByText("Laya Multilingual")).toBeDefined();
    expect(screen.getByText(/1.2 GB/)).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Baixar e ativar" }));
    expect(models.install).not.toHaveBeenCalled();
    expect(
      screen.getByText(/O download de 1.2 GB só começa ao confirmar/),
    ).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Baixar" }));
    expect(models.install).toHaveBeenCalledWith("laya-multilingual", "0.1.0");
  });

  it("cancela a confirmação sem baixar", () => {
    const models = renderManager();
    fireEvent.click(screen.getByRole("button", { name: "Baixar e ativar" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(models.install).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Baixar e ativar" })).toBeDefined();
  });

  it("explica por que o Laya não roda neste build", () => {
    renderManager({
      backend: {
        laya_feature: false,
        model_active: false,
        model_configured: false,
        model_bundled: false,
      },
    });
    expect(
      screen.getByText(/compilado sem o engine Laya/),
    ).toBeDefined();
  });

  it("mostra o modelo empacotado no app", () => {
    renderManager({
      catalog: null,
      recommended: null,
      backend: {
        laya_feature: true,
        model_active: false,
        model_configured: true,
        model_bundled: true,
      },
    });
    expect(screen.getByText("Incluído no app")).toBeDefined();
    expect(
      screen.getByText("Engine Laya incluído; modelo incluído no app."),
    ).toBeDefined();
  });

  it("mostra a versão ativa", () => {
    renderManager({
      installed: [installed({ active: true })],
      active: installed({ active: true }),
    });
    expect(screen.getByText("Ativo: 0.1.0")).toBeDefined();
    expect(screen.getAllByText("Ativa").length).toBeGreaterThan(0);
  });

  it("ativa uma versão instalada e inativa", () => {
    const models = renderManager({ installed: [installed()] });
    fireEvent.click(screen.getAllByRole("button", { name: "Ativar" })[0]!);
    expect(models.activate).toHaveBeenCalledWith("laya-multilingual", "0.1.0");
  });

  it("remove uma versão inativa pelo rodapé", () => {
    const models = renderManager({ installed: [installed()] });
    fireEvent.click(screen.getByRole("button", { name: "Remover" }));
    expect(models.remove).toHaveBeenCalledWith("laya-multilingual", "0.1.0");
  });

  it("mostra erro do catálogo e permite tentar de novo", () => {
    const models = renderManager({
      catalog: null,
      recommended: null,
      error: "offline",
    });
    expect(screen.getByText(/Catálogo indisponível: offline/)).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));
    expect(models.refresh).toHaveBeenCalled();
  });

  it("mostra o progresso do download", () => {
    renderManager({
      progress: {
        model_id: "laya-multilingual",
        version: "0.1.0",
        phase: "downloading",
        file: "model.onnx",
        file_index: 1,
        file_count: 3,
        bytes_done: 600_000_000,
        bytes_total: 1_200_000_000,
        error: null,
      },
    });
    expect(
      screen.getByText("Baixando model.onnx (1/3)…"),
    ).toBeDefined();
    expect(screen.getByRole("progressbar")).toBeDefined();
  });
});
