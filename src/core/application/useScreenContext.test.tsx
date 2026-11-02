import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { defaultSettings } from "../domain/settings";
import { useScreenContext } from "./useScreenContext";

const mocks = vi.hoisted(() => ({
  screenContextSnapshot: vi.fn(),
  setScreenContextInclude: vi.fn(async (_included: boolean) => undefined),
}));

vi.mock("../../adapters/ipc/screenContext", () => ({
  screenContextSnapshot: () => mocks.screenContextSnapshot(),
}));

vi.mock("../../adapters/ipc/settings", () => ({
  setScreenContextInclude: (included: boolean) =>
    mocks.setScreenContextInclude(included),
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: () => Promise.resolve(() => {}),
}));

const active = {
  available: true,
  app: "google-chrome",
  window_title: "Doc",
  preview: "texto",
  captured_at_ms: 1,
};

const settingsWith = (include: boolean) => ({
  ...defaultSettings(),
  screen_context_enabled: true,
  screen_context_include: include,
});

describe("useScreenContext (BR-106)", () => {
  beforeEach(() => {
    mocks.screenContextSnapshot.mockReset();
    mocks.setScreenContextInclude.mockClear();
    mocks.screenContextSnapshot.mockResolvedValue(active);
  });

  it("marcado: consulta ao montar e deriva o estado ativo", async () => {
    const { result } = renderHook(() => useScreenContext(settingsWith(true)));

    await waitFor(() => expect(result.current.state).toBe("active"));
    expect(mocks.screenContextSnapshot).toHaveBeenCalledTimes(1);
    expect(result.current.checked).toBe(true);
  });

  it("desmarcado: NÃO consulta (zero processamento) e fica em pausa", async () => {
    const { result } = renderHook(() => useScreenContext(settingsWith(false)));

    await act(async () => {
      await Promise.resolve();
    });
    expect(mocks.screenContextSnapshot).not.toHaveBeenCalled();
    expect(result.current.state).toBe("paused");
    expect(result.current.event).toBeNull();

    // Reconsulta sob demanda (abrir a barra) também não lê.
    act(() => result.current.refresh());
    await act(async () => {
      await Promise.resolve();
    });
    expect(mocks.screenContextSnapshot).not.toHaveBeenCalled();
  });

  it("desmarcar limpa o contexto e remarcar volta a ler", async () => {
    const { result } = renderHook(() => useScreenContext(settingsWith(true)));
    await waitFor(() => expect(result.current.state).toBe("active"));

    await act(async () => {
      result.current.toggle();
    });
    expect(mocks.setScreenContextInclude).toHaveBeenCalledWith(false);
    await waitFor(() => expect(result.current.checked).toBe(false));
    expect(result.current.state).toBe("paused");
    expect(result.current.event).toBeNull();

    const reads = mocks.screenContextSnapshot.mock.calls.length;
    await act(async () => {
      result.current.toggle();
    });
    expect(mocks.setScreenContextInclude).toHaveBeenLastCalledWith(true);
    await waitFor(() => expect(result.current.state).toBe("active"));
    expect(mocks.screenContextSnapshot.mock.calls.length).toBe(reads + 1);
  });

  it("desmarcar durante uma leitura em voo não agenda nova consulta", async () => {
    // O retry de "sem contexto" (1,5 s) não pode disparar depois de desligar:
    // desmarcado = zero processamento.
    vi.useFakeTimers();
    try {
      let release!: (value: typeof active) => void;
      mocks.screenContextSnapshot.mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            release = resolve;
          }),
      );
      const { result } = renderHook(() => useScreenContext(settingsWith(true)));
      expect(mocks.screenContextSnapshot).toHaveBeenCalledTimes(1);

      act(() => result.current.toggle());
      expect(result.current.state).toBe("paused");

      await act(async () => {
        release({ ...active, available: false });
        await Promise.resolve();
      });
      act(() => {
        vi.advanceTimersByTime(5_000);
      });
      expect(mocks.screenContextSnapshot).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it("cliques seguidos alternam sem depender do round-trip de settings", async () => {
    // Regressão: com `checked` só vindo de settings, dois cliques antes do
    // reload mandariam o mesmo valor (o check não voltava). A fila de
    // gravação mantém a ordem: o último clique é o que fica.
    const { result } = renderHook(() => useScreenContext(settingsWith(true)));
    await act(async () => {
      result.current.toggle();
      result.current.toggle();
    });
    await waitFor(() =>
      expect(
        mocks.setScreenContextInclude.mock.calls.map(([value]) => value),
      ).toEqual([false, true]),
    );
    expect(result.current.checked).toBe(true);
  });
});
