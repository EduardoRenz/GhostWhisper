import { describe, expect, it } from "vitest";
import {
  buildChatTimeline,
  bubblesFromTimeline,
  splitChatTimeline,
  splitSpeechText,
  MAX_SPEECH_BUBBLE_CHARS,
  type ChatBubble,
} from "./chat";
import type { InsightSuggestion, TimelineEntry, TranscriptSegment } from "./transcript";

const segment = (
  overrides: Partial<TranscriptSegment> & { id: string },
): TranscriptSegment => ({
  session_id: "s1",
  speaker: "ME",
  text: "fala",
  start_ms: 0,
  end_ms: 1_000,
  final: true,
  ...overrides,
});

const suggestion = (
  overrides: Partial<InsightSuggestion> & { message_id: string },
): InsightSuggestion => ({
  session_id: "s1",
  text: "sugestão",
  at_ms: 0,
  ...overrides,
});

describe("splitChatTimeline", () => {
  it("separa finalizados (linha temporal) de prévias (em andamento)", () => {
    const { committed, pending } = splitChatTimeline([
      segment({ id: "a", start_ms: 5_000 }),
      segment({ id: "b", start_ms: 1_000 }),
      segment({ id: "c", start_ms: 9_000, final: false, text: "em curso" }),
    ]);

    expect(committed.map((item) => item.id)).toEqual(["b", "a"]);
    expect(pending.map((item) => item.id)).toEqual(["c"]);
  });

  it("não deixa a prévia atrasada reescrever o final do mesmo turno", () => {
    // O bug antigo: a prévia (mesmo id) substituía o texto já fechado.
    const { committed, pending } = splitChatTimeline([
      segment({ id: "t1", text: "fala final" }),
      segment({ id: "t1", text: "prévia velha", final: false }),
    ]);

    expect(committed).toHaveLength(1);
    expect(committed[0]?.text).toBe("fala final");
    expect(pending).toHaveLength(0);
  });

  it("mantém no máximo uma prévia por falante, na ordem você → interlocutor", () => {
    const { pending } = splitChatTimeline([
      segment({ id: "me1", speaker: "ME", start_ms: 1_000, final: false, text: "eu 1" }),
      segment({ id: "me2", speaker: "ME", start_ms: 3_000, final: false, text: "eu 2" }),
      segment({ id: "they1", speaker: "THEY", start_ms: 2_000, final: false, text: "ele" }),
    ]);

    expect(pending.map((item) => item.text)).toEqual(["eu 2", "ele"]);
  });

  it("retratação: final vazio substitui a prévia e não vira bolha (ISS-021)", () => {
    // O pipeline publica um final de texto vazio quando descarta o turno: a bolha
    // "em andamento" com o texto cru precisa sair da conversa.
    const { committed, pending } = splitChatTimeline([
      segment({ id: "t0", start_ms: 1_000, final: false, text: "bom dia pesso" }),
      segment({ id: "t0", start_ms: 1_000, final: true, text: "" }),
    ]);

    expect(pending).toHaveLength(0);

    const { bubbles } = buildChatTimeline([
      segment({ id: "t0", start_ms: 1_000, final: false, text: "bom dia pesso" }),
      segment({ id: "t0", start_ms: 1_000, final: true, text: "" }),
    ]);
    expect(bubbles).toHaveLength(0);
    expect(committed).toHaveLength(1);
  });

  it("intercala falas e sugestões por tempo", () => {
    const { bubbles } = buildChatTimeline(
      [
        segment({ id: "me", start_ms: 1_000, text: "bom dia" }),
        segment({ id: "they", speaker: "THEY", start_ms: 5_000, text: "bom dia!" }),
      ],
      [suggestion({ message_id: "ai", at_ms: 3_000, text: "fale do orçamento" })],
    );

    expect(bubbles.map((item) => item.id)).toEqual(["me", "ai", "they"]);
    expect(bubbles.map((item) => item.kind)).toEqual([
      "speech",
      "suggestion",
      "speech",
    ]);
  });

  it("no mesmo instante, a fala vem antes da sugestão", () => {
    const { bubbles } = buildChatTimeline(
      [segment({ id: "me", start_ms: 2_000, text: "pergunta" })],
      [suggestion({ message_id: "ai", at_ms: 2_000 })],
    );
    expect(bubbles.map((item) => item.kind)).toEqual(["speech", "suggestion"]);
  });

  it("não duplica a sugestão do atalho com a do banco", () => {
    const { bubbles } = buildChatTimeline(
      [],
      [
        suggestion({ message_id: "ai", text: "sugestão" }),
        suggestion({ message_id: "ai", text: "sugestão" }),
      ],
    );
    expect(bubbles).toHaveLength(1);
  });

  it("ignora texto vazio e limita o número de bolhas", () => {
    const segments = Array.from({ length: 5 }, (_, index) =>
      segment({ id: `s${index}`, start_ms: index * 1_000, text: `fala ${index}` }),
    );
    const empty = buildChatTimeline([
      ...segments,
      segment({ id: "blank", start_ms: 9_000, text: "   " }),
    ]);

    expect(empty.bubbles.map((item) => item.id)).toEqual([
      "s0",
      "s1",
      "s2",
      "s3",
      "s4",
    ]);

    const capped = buildChatTimeline(segments, [], 2);
    expect(capped.bubbles.map((item) => item.id)).toEqual(["s3", "s4"]);
  });

  it("expõe bolhas tipadas (fala com falante, sugestão sem)", () => {
    const { bubbles } = buildChatTimeline(
      [segment({ id: "me", text: "oi" })],
      [suggestion({ message_id: "ai" })],
    );
    const speech = bubbles[0] as Extract<ChatBubble, { kind: "speech" }>;
    const ai = bubbles[1] as Extract<ChatBubble, { kind: "suggestion" }>;
    expect(speech.speaker).toBe("ME");
    expect(speech.end_ms).toBe(1_000);
    expect(ai.id).toBe("ai");
  });
});

describe("splitSpeechText", () => {
  it("deixa fala curta em uma bolha só", () => {
    expect(splitSpeechText("bom dia")).toEqual(["bom dia"]);
    expect(splitSpeechText(`  ${"a".repeat(MAX_SPEECH_BUBBLE_CHARS)}  `)).toEqual([
      "a".repeat(MAX_SPEECH_BUBBLE_CHARS),
    ]);
  });

  it("texto em branco não vira bolha", () => {
    expect(splitSpeechText("")).toEqual([]);
    expect(splitSpeechText("   \n  ")).toEqual([]);
  });

  it("quebra no fim da frase antes de estourar o limite", () => {
    expect(splitSpeechText("Primeira frase. Segunda frase.", 20)).toEqual([
      "Primeira frase.",
      "Segunda frase.",
    ]);
  });

  it("nunca corta no meio de uma palavra", () => {
    const text = "palavra ".repeat(40).trim();
    const parts = splitSpeechText(text, 50);

    expect(parts.length).toBeGreaterThan(1);
    for (const part of parts) {
      expect(part.length).toBeLessThanOrEqual(50);
      // Cada parte é uma sequência inteira de palavras.
      expect(part).toMatch(/^palavra( palavra)*$/);
    }
    expect(parts.join(" ")).toBe(text);
  });

  it("corta uma palavra maior que o limite no próprio limite", () => {
    const parts = splitSpeechText("b".repeat(30), 10);
    expect(parts).toEqual(["b".repeat(10), "b".repeat(10), "b".repeat(10)]);
  });

  it("trata quebra de linha como fim de parte", () => {
    const parts = splitSpeechText("linha um\nlinha dois", 12);
    expect(parts).toEqual(["linha um", "linha dois"]);
  });
});

describe("continuação de fala longa", () => {
  const long = `${"a".repeat(100)}. ${"b".repeat(100)}.`;

  it("a fala finalizada continua em bolhas seguintes", () => {
    const { bubbles } = buildChatTimeline([
      segment({ id: "t1", speaker: "THEY", start_ms: 1_000, text: long }),
      segment({ id: "t2", start_ms: 30_000, text: "curta" }),
    ]);

    expect(bubbles.map((item) => item.id)).toEqual(["t1", "t1#1", "t2"]);
    expect(bubbles[0]).toMatchObject({ continues: false, speaker: "THEY" });
    // A continuação mantém o tempo da fala original e marca `continues`.
    expect(bubbles[1]).toMatchObject({ continues: true, at_ms: 1_000 });
    expect((bubbles[1] as Extract<ChatBubble, { kind: "speech" }>).text).toBe(
      `${"b".repeat(100)}.`,
    );
  });

  it("a fala em andamento (prévia) é cortada pela mesma regra", () => {
    // O `pending` sai cru do domínio; quem corta é a view (mesma função).
    const { pending } = buildChatTimeline([
      segment({ id: "t3", speaker: "THEY", final: false, text: long }),
    ]);
    expect(pending).toHaveLength(1);
    expect(splitSpeechText(pending[0]?.text ?? "")).toHaveLength(2);
  });

  it("o histórico corta falas longas em continuações", () => {
    const entries: TimelineEntry[] = [
      { kind: "speech", speaker: "THEY", at_ms: 1_000, text: long },
      { kind: "suggestion", at_ms: 2_000, text: "fale do orçamento" },
    ];
    const bubbles = bubblesFromTimeline(entries);

    expect(bubbles.map((item) => item.id)).toEqual([
      "speech-1000-0",
      "speech-1000-0#1",
      "suggestion-2000-1",
    ]);
    expect(bubbles[1]).toMatchObject({ continues: true, speaker: "THEY" });
    expect(bubbles[2]?.kind).toBe("suggestion");
    // Sugestão não é continuação de nada (a marca só existe em fala).
    expect("continues" in (bubbles[2] as object)).toBe(false);
  });
});
