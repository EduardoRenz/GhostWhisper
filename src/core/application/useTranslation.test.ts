import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTranslation } from "./useTranslation";

const llm = vi.hoisted(() => ({
  translate: vi.fn(async (text: string) => `[${text}]`),
}));

vi.mock("../../adapters/ipc/llm", () => ({
  llmTranslate: (text: string) => llm.translate(text),
}));

describe("useTranslation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    llm.translate.mockImplementation(async (text: string) => `[${text}]`);
  });

  it("traduz sob demanda e reusa o cache", async () => {
    const { result } = renderHook(() => useTranslation());

    let value: string | null = null;
    await act(async () => {
      value = await result.current.translate({ id: "a", text: "bom dia" });
    });

    expect(value).toBe("[bom dia]");
    expect(result.current.translations.a).toBe("[bom dia]");
    expect(result.current.pending.a).toBeUndefined();

    // Segunda vez: cache, sem nova chamada ao provider.
    await act(async () => {
      await result.current.translate({ id: "a", text: "bom dia" });
    });
    expect(llm.translate).toHaveBeenCalledTimes(1);
  });

  it("registra o erro e limpa o pendente", async () => {
    llm.translate.mockRejectedValueOnce(new Error("boom"));
    const { result } = renderHook(() => useTranslation());

    await act(async () => {
      expect(await result.current.translate({ id: "x", text: "t" })).toBeNull();
    });

    expect(result.current.error).toContain("boom");
    expect(result.current.pending.x).toBeUndefined();
    expect(result.current.translations.x).toBeUndefined();
  });

  it("reset limpa o cache e as traduções", async () => {
    const { result } = renderHook(() => useTranslation());
    await act(async () => {
      await result.current.translate({ id: "a", text: "oi" });
    });
    expect(result.current.translations.a).toBeDefined();

    act(() => result.current.reset());
    expect(result.current.translations).toEqual({});
  });
});
