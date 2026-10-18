import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTranslation } from "./useTranslation";
import type { TranscriptSegment } from "../domain/transcript";

const llm = vi.hoisted(() => ({
  translate: vi.fn(async (text: string) => `[${text}]`),
}));

vi.mock("../../adapters/ipc/llm", () => ({
  llmTranslate: (text: string) => llm.translate(text),
}));

const segment = (
  overrides: Partial<TranscriptSegment> & { id: string },
): TranscriptSegment => ({
  session_id: "s1",
  speaker: "ME",
  text: "fala",
  start_ms: 0,
  end_ms: 500,
  final: true,
  ...overrides,
});

describe("useTranslation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("traduz os segmentos finalizados, em fila", async () => {
    renderHook(() =>
      useTranslation(
        [
          segment({ id: "a", text: "bom dia" }),
          segment({ id: "b", text: "tudo bem", start_ms: 1_000 }),
        ],
        true,
      ),
    );

    await waitFor(() => expect(llm.translate).toHaveBeenCalledTimes(2));
    expect(llm.translate).toHaveBeenNthCalledWith(1, "bom dia");
  });

  it("não traduz prévias nem retratação de prévia (ISS-021)", async () => {
    renderHook(() =>
      useTranslation(
        [
          segment({ id: "preview", text: "bom dia pesso", final: false }),
          // Final de texto vazio = retratação: não é fala.
          segment({ id: "retracted", text: "" }),
          segment({ id: "blank", text: "   " }),
        ],
        true,
      ),
    );

    // Nenhuma chamada: nada aqui é fala finalizada com texto.
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(llm.translate).not.toHaveBeenCalled();
  });
});
