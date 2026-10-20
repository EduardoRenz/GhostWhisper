import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CapturePreviewWindow } from "./CapturePreviewWindow";

const capturePending = vi.fn();
const captureClear = vi.fn();

vi.mock("../../adapters/ipc/screen", () => ({
  capturePending: (...args: unknown[]) => capturePending(...args),
  captureClear: (...args: unknown[]) => captureClear(...args),
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: vi.fn(async () => () => {}),
}));

describe("CapturePreviewWindow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturePending.mockResolvedValue({
      mime: "image/jpeg",
      data: "AAA",
      width: 1280,
      height: 720,
    });
    captureClear.mockResolvedValue(undefined);
  });

  it("mostra o print pendente", async () => {
    render(<CapturePreviewWindow />);
    const image = (await screen.findByAltText("Print capturado")) as HTMLImageElement;
    expect(image.src).toContain("data:image/jpeg;base64,AAA");
    expect(screen.getByText("Anexado à pergunta")).toBeTruthy();
  });

  it("o × descarta o print", async () => {
    render(<CapturePreviewWindow />);
    fireEvent.click(await screen.findByLabelText("Remover o print"));
    expect(captureClear).toHaveBeenCalledTimes(1);
  });

  it("sem print pendente não renderiza nada", async () => {
    capturePending.mockResolvedValue(null);
    const { container } = render(<CapturePreviewWindow />);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(container.firstChild).toBeNull();
  });
});
