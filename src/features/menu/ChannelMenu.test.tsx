import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ChannelMenu } from "./ChannelMenu";

const mocks = vi.hoisted(() => ({
  setChannels: vi.fn(),
  start: vi.fn(),
  closeMenu: vi.fn(),
}));

vi.mock("../../adapters/ipc/overlay", () => ({
  closeMenu: mocks.closeMenu,
}));

vi.mock("../../core/application/useSessionState", () => ({
  useSessionState: () => ({
    state: { channels: ["microphone"] },
    busy: false,
    setChannels: mocks.setChannels,
    start: mocks.start,
  }),
}));

describe("ChannelMenu (janela)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.setChannels.mockResolvedValue(undefined);
    mocks.start.mockResolvedValue(undefined);
  });

  it("troca os canais ao vivo e fecha o menu", async () => {
    render(<ChannelMenu />);

    fireEvent.click(
      screen.getByRole("menuitemradio", { name: "Sistema" }),
    );

    expect(mocks.setChannels).toHaveBeenCalledWith(["system"]);
    await waitFor(() => expect(mocks.closeMenu).toHaveBeenCalledTimes(1));
  });

  it("inicia uma nova sessão com a escolha atual", async () => {
    render(<ChannelMenu />);

    fireEvent.click(
      screen.getByRole("menuitem", { name: "Começar a capturar" }),
    );

    expect(mocks.start).toHaveBeenCalledWith(["microphone"]);
    await waitFor(() => expect(mocks.closeMenu).toHaveBeenCalledTimes(1));
  });
});
