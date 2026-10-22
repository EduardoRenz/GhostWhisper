import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useDwellTimer } from "./useDwellTimer";

describe("useDwellTimer", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("dispara uma vez após a duração", () => {
    const onElapsed = vi.fn();
    const { result } = renderHook(() => useDwellTimer(500, onElapsed));

    act(() => result.current.start());
    expect(result.current.active).toBe(true);

    act(() => vi.advanceTimersByTime(499));
    expect(onElapsed).not.toHaveBeenCalled();

    act(() => vi.advanceTimersByTime(1));
    expect(onElapsed).toHaveBeenCalledTimes(1);
    expect(result.current.active).toBe(false);
  });

  it("iniciar de novo enquanto ativo não reinicia o tempo", () => {
    const onElapsed = vi.fn();
    const { result } = renderHook(() => useDwellTimer(500, onElapsed));

    act(() => result.current.start());
    act(() => vi.advanceTimersByTime(300));
    act(() => result.current.start());
    act(() => vi.advanceTimersByTime(199));

    expect(onElapsed).not.toHaveBeenCalled();

    act(() => vi.advanceTimersByTime(1));
    expect(onElapsed).toHaveBeenCalledTimes(1);
  });

  it("stop cancela o disparo", () => {
    const onElapsed = vi.fn();
    const { result } = renderHook(() => useDwellTimer(500, onElapsed));

    act(() => result.current.start());
    act(() => vi.advanceTimersByTime(200));
    act(() => result.current.stop());
    act(() => vi.advanceTimersByTime(500));

    expect(onElapsed).not.toHaveBeenCalled();
    expect(result.current.active).toBe(false);
  });

  it("desligado não arma", () => {
    const onElapsed = vi.fn();
    const { result } = renderHook(() => useDwellTimer(500, onElapsed, false));

    act(() => result.current.start());
    act(() => vi.advanceTimersByTime(500));

    expect(onElapsed).not.toHaveBeenCalled();
    expect(result.current.active).toBe(false);
  });
});
