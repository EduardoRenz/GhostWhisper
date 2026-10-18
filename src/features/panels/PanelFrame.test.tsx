import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const startResizeDragging = vi.fn().mockResolvedValue(undefined);
const panelHide = vi.fn();
const panelStartDrag = vi.fn();

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ startResizeDragging }),
}));

vi.mock("../../adapters/ipc/client", () => ({
  isTauriRuntime: () => true,
}));

vi.mock("../../adapters/ipc/panels", () => ({
  panelHide: (...args: unknown[]) => panelHide(...args),
  panelStartDrag: (...args: unknown[]) => panelStartDrag(...args),
}));

import { PanelFrame } from "./PanelFrame";

describe("PanelFrame", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renderiza o painel e as 8 alças de resize", () => {
    const { container } = render(
      <PanelFrame label="summary" title="Resumo">
        conteúdo
      </PanelFrame>,
    );

    expect(container.querySelector(".panel")).not.toBeNull();
    expect(container.querySelectorAll(".panel__resize")).toHaveLength(8);
    expect(screen.getByText("conteúdo")).toBeDefined();
  });

  it("inicia o resize na direção da alça pressionada", () => {
    const { container } = render(
      <PanelFrame label="summary" title="Resumo">
        x
      </PanelFrame>,
    );

    fireEvent.pointerDown(
      container.querySelector('[data-direction="SouthEast"]')!,
    );

    expect(startResizeDragging).toHaveBeenCalledWith("SouthEast");
  });

  it("esconde a janela ao clicar em Fechar", () => {
    render(
      <PanelFrame label="suggestions" title="Sugestões">
        x
      </PanelFrame>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Fechar" }));
    expect(panelHide).toHaveBeenCalledWith("suggestions");
  });
});
