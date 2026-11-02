import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  screenContextState,
  type ScreenContextEvent,
} from "../../core/domain/screenContext";
import { ContextPill } from "./ContextPill";

function event(overrides: Partial<ScreenContextEvent> = {}): ScreenContextEvent {
  return {
    available: true,
    app: "google-chrome",
    window_title: "API Reference",
    preview: "GET /v1/chat …",
    captured_at_ms: 1_000,
    ...overrides,
  };
}

describe("ContextPill (BR-106)", () => {
  it("não renderiza nada no estado hidden (flag desligada, CA-02.1)", () => {
    const { container } = render(
      <ContextPill state="hidden" event={null} />,
    );
    expect(container.firstChild).toBeNull();
    expect(screen.queryByTestId("context-pill")).toBeNull();
  });

  it("mostra app · título no estado ativo com prévia no tooltip", () => {
    render(<ContextPill state="active" event={event()} />);
    const pill = screen.getByTestId("context-pill");
    expect(pill.textContent).toContain("google-chrome");
    expect(pill.textContent).toContain("API Reference");
    // Tooltip com a prévia do texto (transparência, CA-02.2).
    expect(pill.getAttribute("title")).toContain("GET /v1/chat");
  });

  it("mostra o estado de leitura", () => {
    render(<ContextPill state="loading" event={null} />);
    expect(screen.getByTestId("context-pill").textContent).toContain(
      "Lendo contexto…",
    );
  });

  it("mostra 'Sem contexto de tela' quando a janela não expõe árvore", () => {
    render(
      <ContextPill
        state="empty"
        event={event({ available: false, app: "", window_title: "", preview: "" })}
      />,
    );
    expect(screen.getByTestId("context-pill").textContent).toContain(
      "Sem contexto de tela",
    );
  });

  it("mostra 'Contexto indisponível' quando o AT-SPI falha", () => {
    render(<ContextPill state="unavailable" event={null} />);
    expect(screen.getByTestId("context-pill").textContent).toContain(
      "Contexto indisponível",
    );
  });

  it("anuncia mudanças de estado via aria-live (RNF-06)", () => {
    render(<ContextPill state="active" event={event()} />);
    const pill = screen.getByTestId("context-pill");
    expect(pill.getAttribute("aria-live")).toBe("polite");
    expect(pill.getAttribute("role")).toBe("status");
    expect(pill.getAttribute("aria-label")).toContain("google-chrome");
  });

  // --- Checkbox (BR-106) ---

  it("a pill inteira é o checkbox: um clique alterna", () => {
    const onToggle = vi.fn();
    render(
      <ContextPill state="active" event={event()} checked onToggle={onToggle} />,
    );
    const pill = screen.getByTestId("context-pill");
    expect(pill.getAttribute("role")).toBe("checkbox");
    expect(pill.getAttribute("aria-checked")).toBe("true");
    // O aria-label diz o estado e a ação (incluir/excluir).
    expect(pill.getAttribute("aria-label")).toContain("incluído");

    // Um clique = uma alternância (sem alvo separado).
    fireEvent.click(pill);
    expect(onToggle).toHaveBeenCalledTimes(1);
    // A prévia continua no tooltip (transparência não some com o checkbox).
    expect(pill.getAttribute("title")).toContain("GET /v1/chat");
  });

  it("desmarcado mostra aria-checked=false e fica em pausa (sem contexto)", () => {
    render(
      <ContextPill
        state="paused"
        event={null}
        checked={false}
        onToggle={vi.fn()}
      />,
    );
    const pill = screen.getByTestId("context-pill");
    expect(pill.getAttribute("role")).toBe("checkbox");
    expect(pill.getAttribute("aria-checked")).toBe("false");
    expect(pill.getAttribute("aria-label")).toContain("desativado");
    expect(pill.textContent).toContain("Contexto desativado");
    expect(pill.className).toContain("askbar__context--unchecked");
  });

  it("sem onToggle nunca vira checkbox; com onToggle, checkbox em qualquer estado", () => {
    // Sem toggle → informativo (status) mesmo ativo.
    const { rerender } = render(
      <ContextPill state="active" event={event()} checked />,
    );
    expect(screen.getByTestId("context-pill").getAttribute("role")).toBe(
      "status",
    );
    // Com toggle, a OPÇÃO aparece mesmo sem contexto (pedido do usuário) —
    // a preferência é visível em todos os estados.
    rerender(
      <ContextPill state="empty" event={null} checked={false} onToggle={vi.fn()} />,
    );
    const pill = screen.getByTestId("context-pill");
    expect(pill.getAttribute("role")).toBe("checkbox");
    expect(pill.getAttribute("aria-checked")).toBe("false");
    expect(pill.textContent).toContain("Sem contexto");
    expect(pill.getAttribute("aria-label")).toContain("não incluído");
    // Indisponível também.
    rerender(
      <ContextPill state="unavailable" event={null} checked onToggle={vi.fn()} />,
    );
    expect(screen.getByTestId("context-pill").getAttribute("role")).toBe(
      "checkbox",
    );
  });

  it("estado 'empty' nunca mostra app/título antigos", () => {
    render(
      <ContextPill
        state="empty"
        event={event({ available: false, app: "", window_title: "" })}
      />,
    );
    expect(screen.getByTestId("context-pill").textContent).not.toContain(
      "google-chrome",
    );
  });
});

describe("screenContextState (derivação do estado)", () => {
  it("flag desligada → hidden, com ou sem evento", () => {
    expect(screenContextState(false, null)).toBe("hidden");
    expect(screenContextState(false, event())).toBe("hidden");
  });

  it("flag ligada sem evento → loading (a consulta ainda voltou)", () => {
    expect(screenContextState(true, null)).toBe("loading");
  });

  it("checkbox desmarcado → paused, mesmo com evento antigo e sem consulta", () => {
    // BR-106: desmarcado, nada é lido; o evento anterior não pode voltar.
    expect(screenContextState(true, event(), false)).toBe("paused");
    expect(screenContextState(true, null, false)).toBe("paused");
    // A flag mestre vence: desligada nem renderiza (hidden).
    expect(screenContextState(false, event(), false)).toBe("hidden");
  });

  it("evento indisponível → empty; com failed → unavailable (ISS-092)", () => {
    expect(screenContextState(true, event())).toBe("active");
    expect(
      screenContextState(true, event({ available: false })),
    ).toBe("empty");
    // Falha de leitura (timeout/D-Bus) vence `available:false` simples:
    // "Contexto indisponível" ≠ "Sem contexto de tela".
    expect(
      screenContextState(true, event({ available: false, failed: true })),
    ).toBe("unavailable");
    expect(
      screenContextState(true, event({ available: true, failed: true })),
    ).toBe("unavailable");
  });

  it("nunca volta a loading depois da primeira consulta (ISS-092)", () => {
    // Regressão: null eterno = 'Lendo contexto...' infinito. Com contrato novo,
    // o command sempre devolve DTO com a flag ligada; null só = loading.
    expect(screenContextState(true, null)).toBe("loading");
    const answered = screenContextState(
      true,
      event({ available: false, failed: true }),
    );
    expect(answered).toBe("unavailable");
  });
});
