import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TranslationView } from "./TranslationView";
import type { TranscriptSegment } from "../../core/domain/transcript";

const segment = (id: string, text: string): TranscriptSegment => ({
  id,
  session_id: "s1",
  speaker: "ME",
  text,
  start_ms: 0,
  end_ms: 1000,
  final: true,
});

describe("TranslationView", () => {
  it("mostra mensagem quando não há traduções", () => {
    render(
      <TranslationView segments={[]} translations={{}} targetLanguage="en" />,
    );
    expect(screen.getByText(/tradução \(en\)/i)).toBeDefined();
  });

  it("mostra original e tradução com botão copiar", () => {
    render(
      <TranslationView
        segments={[segment("1", "bom dia")]}
        translations={{ "1": "good morning" }}
        targetLanguage="en"
      />,
    );
    expect(screen.getByText("bom dia")).toBeDefined();
    expect(screen.getByText("good morning")).toBeDefined();
    expect(screen.getByRole("button", { name: "Copiar" })).toBeDefined();
  });
});
