import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TranscriptView } from "./TranscriptView";
import type { TranscriptSegment } from "../../core/domain/transcript";

const segment = (
  id: string,
  speaker: "ME" | "THEY",
  text: string,
): TranscriptSegment => ({
  id,
  session_id: "s1",
  speaker,
  text,
  start_ms: 0,
  end_ms: 1000,
  final: true,
});

describe("TranscriptView", () => {
  it("mostra vazio quando não há segmentos", () => {
    render(<TranscriptView segments={[]} />);
    expect(screen.getByText(/nenhuma fala/i)).toBeDefined();
  });

  it("rotula os falantes e mostra o texto", () => {
    render(
      <TranscriptView
        segments={[segment("1", "ME", "olá"), segment("2", "THEY", "oi")]}
      />,
    );
    expect(screen.getByText("Você")).toBeDefined();
    expect(screen.getByText("Interlocutor")).toBeDefined();
    expect(screen.getByText("olá")).toBeDefined();
    expect(screen.getByText("oi")).toBeDefined();
  });

  it("mostra o tempo de cada fala desde o início da sessão", () => {
    render(
      <TranscriptView
        segments={[
          { ...segment("1", "ME", "olá"), start_ms: 65_000 },
          { ...segment("2", "THEY", "oi"), start_ms: 3_725_000 },
        ]}
      />,
    );

    expect(screen.getByText("01:05")).toBeDefined();
    expect(screen.getByText("1:02:05")).toBeDefined();
  });
});
