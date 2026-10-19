import { describe, expect, it } from "vitest";
import {
  formatClock,
  formatClockLong,
  formatDateTime,
  formatMessageClock,
  mergeHistory,
  mergeSegment,
  sessionDuration,
  sessionTitle,
  timelineLabel,
  toSuggestion,
  type SessionSummary,
  type TranscriptSegment,
} from "./transcript";

const segment = (
  id: string,
  startMs: number,
  text: string,
  final = true,
): TranscriptSegment => ({
  id,
  session_id: "s1",
  speaker: "ME",
  text,
  start_ms: startMs,
  end_ms: startMs + 500,
  final,
});

describe("formatClock", () => {
  it("formata a linha temporal em mm:ss", () => {
    expect(formatClock(0)).toBe("00:00");
    expect(formatClock(1_500)).toBe("00:01");
    expect(formatClock(65_000)).toBe("01:05");
    expect(formatClock(59_999)).toBe("00:59");
  });

  it("inclui horas quando a sessão passa de uma hora", () => {
    expect(formatClock(3_600_000)).toBe("1:00:00");
    expect(formatClock(3_725_000)).toBe("1:02:05");
  });

  it("formatClockLong sempre usa HH:MM:SS (timer do notch)", () => {
    expect(formatClockLong(0)).toBe("00:00:00");
    expect(formatClockLong(65_000)).toBe("00:01:05");
    expect(formatClockLong(3_725_000)).toBe("01:02:05");
    expect(formatClockLong(-5)).toBe("00:00:00");
  });

  it("nunca mostra tempo negativo", () => {
    expect(formatClock(-500)).toBe("00:00");
  });
});

describe("mergeSegment", () => {
  it("substitui a prévia pelo final com o mesmo id", () => {
    const preview = segment("s1-0-t0", 0, "olá mun", false);
    const final = segment("s1-0-t0", 0, "olá mundo");

    const afterPreview = mergeSegment([], preview);
    expect(afterPreview).toHaveLength(1);

    const afterFinal = mergeSegment(afterPreview, final);
    expect(afterFinal).toHaveLength(1);
    expect(afterFinal[0]?.text).toBe("olá mundo");
    expect(afterFinal[0]?.final).toBe(true);
  });

  it("acrescenta turnos distintos em ordem de chegada", () => {
    const merged = mergeSegment(
      mergeSegment([], segment("s1-0-t0", 0, "primeiro")),
      segment("s1-0-t1", 1_000, "segundo"),
    );
    expect(merged.map((item) => item.text)).toEqual(["primeiro", "segundo"]);
  });

  it("respeita o teto de segmentos em tela", () => {
    let list: TranscriptSegment[] = [];
    for (let index = 0; index < 5; index += 1) {
      list = mergeSegment(list, segment(`s1-0-t${index}`, index * 1_000, `f${index}`), 3);
    }
    expect(list.map((item) => item.text)).toEqual(["f2", "f3", "f4"]);
  });
});

describe("mergeHistory", () => {
  it("mescla o histórico com os eventos ao vivo sem duplicar", () => {
    // O banco devolve o final que também chegou por evento.
    const persisted = [segment("s1-0-t0", 0, "primeira")];
    const live = [
      segment("s1-0-t0", 0, "primeira"),
      segment("s1-0-t1", 1_000, "segunda"),
    ];

    const merged = mergeHistory(persisted, live);

    expect(merged.map((item) => item.text)).toEqual(["primeira", "segunda"]);
  });

  it("o final do banco ganha da prévia que ainda está em tela", () => {
    const persisted = [segment("s1-0-t0", 0, "olá mundo")];
    const live = [segment("s1-0-t0", 0, "olá mun", false)];

    const merged = mergeHistory(persisted, live);

    expect(merged).toHaveLength(1);
    expect(merged[0]?.text).toBe("olá mundo");
    expect(merged[0]?.final).toBe(true);
  });

  it("ordena pela linha temporal e corta o excedente mais antigo", () => {
    const persisted = [
      segment("s1-0-t2", 2_000, "terceira"),
      segment("s1-0-t0", 0, "primeira"),
    ];
    const live = [segment("s1-0-t1", 1_000, "segunda")];

    const merged = mergeHistory(persisted, live, 2);

    expect(merged.map((item) => item.text)).toEqual(["segunda", "terceira"]);
  });
});

describe("histórico de sessões", () => {
  const session = (partial: Partial<SessionSummary> = {}): SessionSummary => ({
    id: "s1",
    started_at_ms: new Date(2026, 0, 2, 3, 4).getTime(),
    ended_at_ms: null,
    subject: null,
    status: "ended",
    ...partial,
  });

  it("usa 'Sessão sem título' quando não há título", () => {
    expect(sessionTitle(session())).toBe("Sessão sem título");
    expect(sessionTitle(session({ subject: "  " }))).toBe("Sessão sem título");
    expect(sessionTitle(session({ subject: "Reunião de sexta" }))).toBe(
      "Reunião de sexta",
    );
  });

  it("formata data e duração", () => {
    expect(formatDateTime(new Date(2026, 0, 2, 3, 4).getTime())).toBe(
      "02/01/2026 03:04",
    );
    const started = 1_000_000;
    expect(
      sessionDuration(session({ started_at_ms: started, ended_at_ms: started + 65_000 })),
    ).toBe("01:05");
    // Em andamento: usa o "agora" informado.
    expect(
      sessionDuration(
        session({ started_at_ms: started, ended_at_ms: null }),
        started + 3_725_000,
      ),
    ).toBe("1:02:05");
  });

  it("rotula falas e sugestões", () => {
    expect(
      timelineLabel({ at_ms: 0, kind: "speech", speaker: "ME", text: "oi" }),
    ).toBe("Microfone");
    expect(
      timelineLabel({
        at_ms: 0,
        kind: "speech",
        speaker: "THEY",
        text: "oi",
      }),
    ).toBe("Outros");
    expect(timelineLabel({ at_ms: 0, kind: "suggestion", text: "sug" })).toBe(
      "Sugestão da IA",
    );
  });

  it("converte a mensagem gravada em sugestão exibida", () => {
    expect(
      toSuggestion({
        id: "m1",
        session_id: "s1",
        role: "assistant",
        content: "faça X",
        timestamp_ms: 9_000,
      }),
    ).toEqual({
      message_id: "m1",
      session_id: "s1",
      text: "faça X",
      at_ms: 9_000,
    });
  });

  it("formatMessageClock usa a hora de parede e cai em mm:ss sem o início", () => {
    // O print mostra 10:42:15/10:43:02 (BR-040).
    const started = new Date(2026, 9, 19, 10, 42, 15).getTime();
    expect(formatMessageClock(0, started)).toBe("10:42:15");
    expect(formatMessageClock(47_000, started)).toBe("10:43:02");
    expect(formatMessageClock(65_000)).toBe("01:05");
    expect(formatMessageClock(65_000, null)).toBe("01:05");
  });
});
