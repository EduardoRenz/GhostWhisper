import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MenuWindow } from "./MenuWindow";

const mocks = vi.hoisted(() => ({
  menuState: vi.fn(),
  closeMenu: vi.fn(),
  resizeMenu: vi.fn(),
  handlers: {} as Record<string, (payload: unknown) => void>,
  focus: undefined as
    | ((event: { payload: boolean }) => void)
    | undefined,
}));

vi.mock("../../adapters/ipc/overlay", () => ({
  menuState: mocks.menuState,
  closeMenu: mocks.closeMenu,
  resizeMenu: mocks.resizeMenu,
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: (
    event: string,
    handler: (payload: unknown) => void,
  ): Promise<() => void> => {
    mocks.handlers[event] = handler;
    return Promise.resolve(() => {});
  },
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onFocusChanged: (
      handler: (event: { payload: boolean }) => void,
    ): Promise<() => void> => {
      mocks.focus = handler;
      return Promise.resolve(() => {});
    },
  }),
}));

vi.mock("./AgentMenu", () => ({
  AgentMenu: () => <div className="menu-list">agentes</div>,
}));

vi.mock("./ChannelMenu", () => ({
  ChannelMenu: () => <div>canais</div>,
}));

describe("MenuWindow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.handlers = {};
    mocks.focus = undefined;
    mocks.menuState.mockResolvedValue(null);
  });

  it("abre o menu de agentes pelo evento do core", async () => {
    render(<MenuWindow />);
    await waitFor(() => expect(mocks.handlers.menu_show).toBeDefined());

    act(() => mocks.handlers.menu_show?.("agent"));

    expect(await screen.findByText("agentes")).toBeDefined();
  });

  it("troca para o menu de canais", async () => {
    render(<MenuWindow />);
    await waitFor(() => expect(mocks.handlers.menu_show).toBeDefined());

    act(() => mocks.handlers.menu_show?.("channel"));

    expect(await screen.findByText("canais")).toBeDefined();
  });

  it("pede a altura dinâmica medida pelo conteúdo", async () => {
    Object.defineProperty(HTMLElement.prototype, "scrollHeight", {
      configurable: true,
      get: () => 120,
    });
    render(<MenuWindow />);
    await waitFor(() => expect(mocks.handlers.menu_show).toBeDefined());

    act(() => mocks.handlers.menu_show?.("agent"));

    // 120 do conteúdo + 14 (borda do card + padding da janela).
    await waitFor(() => expect(mocks.resizeMenu).toHaveBeenCalledWith(134));
  });

  it("Escape fecha a janela", async () => {
    render(<MenuWindow />);
    await waitFor(() => expect(mocks.handlers.menu_show).toBeDefined());
    act(() => mocks.handlers.menu_show?.("agent"));

    act(() => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    });

    expect(mocks.closeMenu).toHaveBeenCalled();
  });

  it("perder o foco depois de focado fecha a janela", async () => {
    render(<MenuWindow />);
    await waitFor(() => expect(mocks.focus).toBeDefined());

    act(() => mocks.focus?.({ payload: true }));
    act(() => mocks.focus?.({ payload: false }));

    expect(mocks.closeMenu).toHaveBeenCalled();
  });

  it("ignora payload inválido", async () => {
    render(<MenuWindow />);
    await waitFor(() => expect(mocks.handlers.menu_show).toBeDefined());

    act(() => mocks.handlers.menu_show?.("bogus"));

    expect(screen.queryByText("agentes")).toBeNull();
    expect(screen.queryByText("canais")).toBeNull();
  });
});
