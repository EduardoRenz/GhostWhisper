import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LogsSettings } from "./LogsSettings";
import { logsRecent } from "../../adapters/ipc/logs";

vi.mock("../../adapters/ipc/logs", () => ({
  logsPath: vi.fn(async () => "/tmp/ghostwhisper.log"),
  logsRecent: vi.fn(async () => "linha 1\nlinha 2"),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  revealItemInDir: vi.fn(async () => {}),
}));

describe("LogsSettings", () => {
  it("mostra as últimas linhas e permite atualizar", async () => {
    const { container } = render(<LogsSettings />);
    await waitFor(() => {
      expect(container.querySelector(".settings__logs")?.textContent).toContain(
        "linha 2",
      );
    });
    expect(screen.getByText("Logs")).toBeDefined();
    expect(screen.getByText("Arquivo local; nada é enviado.")).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Atualizar" }));
    await waitFor(() => {
      expect(vi.mocked(logsRecent)).toHaveBeenCalledTimes(2);
    });
  });
});
