import { describe, expect, it } from "vitest";
import {
  buildChatRows,
  buildChatTimeline,
  bubblesFromTimeline,
  splitChatTimeline,
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

  it("a fala em andamento fica no lugar do tempo, não no fim da lista", () => {
    // Regressão do relato de 27/10: o turno longo do outro lado ainda está em
    // processamento; a fala do ME que veio depois não pode aparecer antes dele.
    const { bubbles, pending } = buildChatTimeline([
      segment({
        id: "they-antes",
        speaker: "THEY",
        start_ms: 1_000,
        end_ms: 2_000,
        text: "antes",
      }),
      segment({
        id: "they-em-curso",
        speaker: "THEY",
        start_ms: 3_000,
        end_ms: 9_000,
        final: false,
        text: "em curso",
      }),
      segment({
        id: "me-depois",
        speaker: "ME",
        start_ms: 6_000,
        end_ms: 7_000,
        text: "depois",
      }),
    ]);

    const rows = buildChatRows(bubbles, pending);
    expect(
      rows.map((row) =>
        row.kind === "bubble" ? row.bubble.id : row.segment.id,
      ),
    ).toEqual(["they-antes", "they-em-curso", "me-depois"]);
  });

  it("intercala a fala em andamento no bloco finalizado que ela interrompeu", () => {
    // Relato de 28/10: o bloco do interlocutor já estava formado e a fala em
    // andamento (Streaming) ficava atrás dele, mesmo tendo começado no meio.
    // A prévia precisa servir de fronteira para o bloco finalizado (ISS-082).
    const theyText = `${"fala longa do interlocutor. ".repeat(8)}`.trim();
    const { bubbles, pending } = buildChatTimeline([
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 1_000,
        end_ms: 21_000,
        text: theyText,
      }),
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 11_000,
        end_ms: 13_000,
        final: false,
        text: "minha fala em andamento",
      }),
    ]);

    const rows = buildChatRows(bubbles, pending);
    expect(
      rows.map((row) =>
        row.kind === "bubble" ? row.bubble.id : row.segment.id,
      ),
    ).toEqual(["they", "me", "they#ov1"]);
  });

  it("intercala a prévia real de 28/10 no bloco de load balancer", () => {
    // Sessão `9e6d8300…`: o interlocutor já tinha o bloco formado (101–121 s)
    // e o ME começou a falar em 119,462 s. A prévia "Hmmmm…" tem de aparecer
    // entre as duas partes do bloco, não depois dele (ISS-082).
    const theyText = "Perfeito, é exatamente isso. Usar uma chave de idem potência e garantir unicidade no banco para não duplicar a transação. Boa resposta! Agora, uma de arquitetura. Onde você colocaria um load balancer se sua aplicação tem várias instâncias de back-end?";
    const { bubbles, pending } = buildChatTimeline([
      segment({
        id: "they6",
        speaker: "THEY",
        start_ms: 101_053,
        end_ms: 121_013,
        text: theyText,
      }),
      segment({
        id: "me15",
        speaker: "ME",
        start_ms: 119_462,
        end_ms: 123_595,
        final: false,
        text: "Hmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmm",
      }),
    ]);

    const rows = buildChatRows(bubbles, pending);
    expect(
      rows.map((row) =>
        row.kind === "bubble" ? row.bubble.id : row.segment.id,
      ),
    ).toEqual(["they6", "me15", "they6#ov1"]);
  });

  it("não parte o bloco finalizado para uma prévia curta ou posterior", () => {
    const theyText = `${"fala longa do interlocutor. ".repeat(8)}`.trim();
    const short = buildChatTimeline([
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 1_000,
        end_ms: 21_000,
        text: theyText,
      }),
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 11_000,
        end_ms: 12_000,
        final: false,
        text: "hum",
      }),
    ]);
    expect(
      buildChatRows(short.bubbles, short.pending).map((row) =>
        row.kind === "bubble" ? row.bubble.id : row.segment.id,
      ),
    ).toEqual(["they", "me"]);

    const later = buildChatTimeline([
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 1_000,
        end_ms: 21_000,
        text: theyText,
      }),
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 22_000,
        end_ms: 24_000,
        final: false,
        text: "depois do bloco",
      }),
    ]);
    expect(
      buildChatRows(later.bubbles, later.pending).map((row) =>
        row.kind === "bubble" ? row.bubble.id : row.segment.id,
      ),
    ).toEqual(["they", "me"]);
  });

  it("mantém a posição da fala quando a prévia vira final", () => {
    const theyText = `${"fala longa do interlocutor. ".repeat(8)}`.trim();
    const pending = buildChatTimeline([
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 1_000,
        end_ms: 21_000,
        text: theyText,
      }),
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 11_000,
        end_ms: 13_000,
        final: false,
        text: "minha fala",
      }),
    ]);
    const final = buildChatTimeline([
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 1_000,
        end_ms: 21_000,
        text: theyText,
      }),
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 11_000,
        end_ms: 13_000,
        text: "minha fala",
      }),
    ]);

    const ids = (rows: ReturnType<typeof buildChatRows>) =>
      rows.map((row) => (row.kind === "bubble" ? row.bubble.id : row.segment.id));
    expect(ids(buildChatRows(pending.bubbles, pending.pending))).toEqual([
      "they",
      "me",
      "they#ov1",
    ]);
    expect(ids(buildChatRows(final.bubbles, final.pending))).toEqual([
      "they",
      "me",
      "they#ov1",
    ]);
  });

  it("ao finalizar, a fala não muda de posição (mesmo start_ms)", () => {
    const before = buildChatRows(
      [],
      [
        segment({
          id: "me",
          speaker: "ME",
          start_ms: 5_000,
          final: false,
          text: "em curso",
        }),
      ],
    );
    const after = buildChatRows(
      [
        {
          kind: "speech",
          id: "me",
          speaker: "ME",
          text: "fechada",
          at_ms: 5_000,
        },
      ],
      [],
    );

    const at = (rows: ReturnType<typeof buildChatRows>) =>
      rows[0]?.kind === "pending"
        ? rows[0].segment.start_ms
        : rows[0]?.bubble.at_ms;
    expect(at(before)).toBe(at(after));
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

describe("fala longa (sem divisão por tamanho)", () => {
  const long = `${"a".repeat(100)}. ${"b".repeat(100)}. ${"c".repeat(100)}.`;

  it("a fala finalizada continua em uma bolha só, com o texto inteiro", () => {
    // Regressão da ISS-065: o teto de 180 caracteres quebrava uma fala em
    // várias bolhas e dava a impressão de várias mensagens ainda em processamento.
    const { bubbles } = buildChatTimeline([
      segment({ id: "t1", speaker: "THEY", start_ms: 1_000, text: long }),
      segment({ id: "t2", start_ms: 30_000, text: "curta" }),
    ]);

    expect(bubbles.map((item) => item.id)).toEqual(["t1", "t2"]);
    expect(bubbles[0]).toMatchObject({ speaker: "THEY", text: long });
    expect("continues" in (bubbles[0] as object)).toBe(false);
    expect("source_text" in (bubbles[0] as object)).toBe(false);
  });

  it("a prévia longa fica em uma única linha em andamento, com o texto cru", () => {
    const { pending, bubbles } = buildChatTimeline([
      segment({ id: "t3", speaker: "THEY", final: false, text: long }),
    ]);
    expect(bubbles).toHaveLength(0);
    expect(pending).toHaveLength(1);
    expect(pending[0]?.text).toBe(long);

    const rows = buildChatRows(bubbles, pending);
    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({ kind: "pending" });
    expect(
      rows[0]?.kind === "pending" ? rows[0].segment.text : null,
    ).toBe(long);
  });

  it("o histórico mantém a fala longa em uma bolha só", () => {
    const entries: TimelineEntry[] = [
      { kind: "speech", speaker: "THEY", at_ms: 1_000, text: long },
      { kind: "suggestion", at_ms: 2_000, text: "fale do orçamento" },
    ];
    const bubbles = bubblesFromTimeline(entries);

    expect(bubbles.map((item) => item.id)).toEqual([
      "speech-1000-0",
      "suggestion-2000-1",
    ]);
    expect(bubbles[0]).toMatchObject({ kind: "speech", text: long });
    expect(bubbles[1]?.kind).toBe("suggestion");
  });

  // Fala longa o bastante para o corte de sobreposição valer (≥80 caracteres).
  const longThey =
    "um dois três quatro cinco seis sete oito nove dez. onze doze treze catorze quinze dezesseis dezessete dezoito dezenove vinte vinte e um vinte e dois vinte e três.";

  it("corta a fala longa no início do outro falante e coloca a nova no meio", () => {
    const { bubbles } = buildChatTimeline([
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 0,
        end_ms: 20_000,
        text: longThey,
      }),
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 10_000,
        end_ms: 20_000,
        text: "minha fala",
      }),
    ]);
    const speech = bubbles.filter(
      (item): item is Extract<ChatBubble, { kind: "speech" }> =>
        item.kind === "speech",
    );

    expect(speech.map((item) => item.id)).toEqual(["they", "me", "they#ov1"]);
    expect(speech[0]?.at_ms).toBe(0);
    expect(speech[1]?.at_ms).toBe(10_000);
    // 1 ms depois do início do outro, para o desempate colocar a fala no meio.
    expect(speech[2]?.at_ms).toBe(10_001);
    expect(speech[0]?.text).toMatch(/^um /);
    expect(speech[2]?.text).toMatch(/três\.$/);
  });

  it("a intercalação é simétrica (microfone longo e outro falante no meio)", () => {
    const { bubbles } = buildChatTimeline([
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 0,
        end_ms: 20_000,
        text: longThey,
      }),
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 10_000,
        end_ms: 20_000,
        text: "fala do outro",
      }),
    ]);
    const speech = bubbles.filter(
      (item): item is Extract<ChatBubble, { kind: "speech" }> =>
        item.kind === "speech",
    );

    expect(speech.map((item) => item.id)).toEqual(["me", "they", "me#ov1"]);
    expect(speech[1]?.at_ms).toBe(10_000);
    expect(speech[2]?.at_ms).toBe(10_001);
  });

  it("não parte uma fala curta só porque o outro canal começou", () => {
    // Regressão da ISS-075: "vamos lá então vamos começar" (28c, 4s) virava
    // "vamos lá então" + "vamos começar" só porque o sistema iniciou no meio.
    const { bubbles } = buildChatTimeline([
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 0,
        end_ms: 4_000,
        text: "vamos lá então vamos começar",
      }),
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 2_000,
        end_ms: 20_000,
        text: longThey,
      }),
    ]);
    const mine = bubbles.find(
      (item) => item.kind === "speech" && item.id === "me",
    );
    expect(mine?.kind).toBe("speech");
    expect(mine?.kind === "speech" ? mine.text : "").toBe(
      "vamos lá então vamos começar",
    );
    expect(bubbles.some((item) => item.id === "me#ov1")).toBe(false);
  });

  it("ignora interjeição curta ao intercalar (e não deixa sobras de 1–5 caracteres)", () => {
    // "Ei, ei, ei!" (2,4s) partia a fala do outro canal em "Ei," + "ei, ei!".
    const { bubbles } = buildChatTimeline([
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 0,
        end_ms: 20_000,
        text: longThey,
      }),
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 10_000,
        end_ms: 11_400,
        text: "Ei, ei, ei!",
      }),
    ]);
    const speech = bubbles.filter(
      (item): item is Extract<ChatBubble, { kind: "speech" }> =>
        item.kind === "speech",
    );
    expect(speech.map((item) => item.id)).toEqual(["they", "me"]);
    expect(speech[0]?.text).toBe(longThey);
  });

  it("mantém a fala inteira quando o corte deixaria um pedaço sem contexto", () => {
    // "o homem pisou na lua pela primeira" + "vez." (4c) era o caso real.
    const withTail = `${longThey} Fim.`;
    const { bubbles } = buildChatTimeline([
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 0,
        end_ms: 20_000,
        text: withTail,
      }),
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 19_000,
        end_ms: 24_000,
        text: "interrupção",
      }),
    ]);
    const speech = bubbles.filter(
      (item): item is Extract<ChatBubble, { kind: "speech" }> =>
        item.kind === "speech",
    );
    expect(speech.map((item) => item.id)).toEqual(["they", "me"]);
    expect(speech[0]?.text).toBe(withTail);
  });

  it("não corta a fala do outro quando ela terminou antes da fala do ME", () => {
    // Sessão real `5984de1d…`: o fim do turno (buffer) era 127,937 s, mas a fala
    // do THEY terminou em ~122,5 s; o ME começou em 122,765 s. Com o fim **real
    // da fala** (ISS-079) não há sobreposição e a ordem é THEY → ME.
    const { bubbles } = buildChatTimeline([
      segment({
        id: "they",
        speaker: "THEY",
        start_ms: 113_985,
        end_ms: 122_500,
        text: "essa solução para garantir consistência dos dados e evitar problemas de concorrência. Responda como se estivesse em uma entrevista técnica. Depois, vou avaliar sua resposta e sugerir melhorias.",
      }),
      segment({
        id: "me",
        speaker: "ME",
        start_ms: 122_765,
        end_ms: 127_373,
        text: "Está bom, deixa eu pensar aqui então.",
      }),
    ]);

    expect(bubbles.map((item) => item.id)).toEqual(["they", "me"]);
    expect(bubbles.some((item) => item.id === "they#ov1")).toBe(false);
    const rows = buildChatRows(bubbles);
    expect(
      rows.map((row) => (row.kind === "bubble" ? row.bubble.id : row.segment.id)),
    ).toEqual(["they", "me"]);
  });

  it("não corta falas que não se sobrepõem", () => {
    const { bubbles } = buildChatTimeline([
      segment({ id: "a", speaker: "THEY", start_ms: 0, end_ms: 5_000, text: "antes" }),
      segment({ id: "b", speaker: "ME", start_ms: 5_000, end_ms: 6_000, text: "depois" }),
    ]);
    expect(bubbles.map((item) => item.id)).toEqual(["a", "b"]);
  });

  it("o histórico também intercala pela sobreposição quando tem end_ms", () => {
    const entries: TimelineEntry[] = [
      {
        kind: "speech",
        speaker: "THEY",
        at_ms: 0,
        end_ms: 20_000,
        text: longThey,
      },
      { kind: "speech", speaker: "ME", at_ms: 10_000, end_ms: 20_000, text: "minha fala" },
    ];
    const bubbles = bubblesFromTimeline(entries);

    expect(bubbles.map((item) => item.id)).toEqual([
      "speech-0-0",
      "speech-10000-1",
      "speech-0-0#ov1",
    ]);
    expect(bubbles[1]?.at_ms).toBe(10_000);
    expect(bubbles[2]?.at_ms).toBe(10_001);
  });
});
