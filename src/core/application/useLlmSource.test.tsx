import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useLlmSource, type LlmSourceUsage } from "./useLlmSource";

const events = vi.hoisted(() => ({
  handlers: [] as ((payload: LlmSourceUsage) => void)[],
}));

vi.mock("../../adapters/ipc/events", () => ({
  subscribe: vi.fn(
    async (_event: string, handler: (payload: LlmSourceUsage) => void) => {
      events.handlers.push(handler);
      return () => {
        events.handlers = events.handlers.filter((item) => item !== handler);
      };
    },
  ),
}));

const usage = (scope: string, model: string): LlmSourceUsage => ({
  source_id: "fonte-1",
  label: "DeepSeek",
  provider: "deepseek",
  model,
  scope,
  is_fallback: false,
  latency_ms: 120,
});

/** Componente de teste: expõe o que o hook devolve. */
function Probe({ scope }: { scope?: string }) {
  const value = useLlmSource(scope);
  return (
    <span data-testid="usage">{value ? `${value.scope}|${value.model}` : "nenhum"}</span>
  );
}

describe("useLlmSource", () => {
  beforeEach(() => {
    events.handlers = [];
  });

  it("começa sem uso", async () => {
    render(<Probe />);
    expect((await screen.findByTestId("usage")).textContent).toBe("nenhum");
  });

  it("guarda o último uso", async () => {
    render(<Probe />);
    await act(async () => {
      events.handlers[0]?.(usage("default", "gpt-4o-mini"));
    });
    expect(screen.getByTestId("usage").textContent).toBe("default|gpt-4o-mini");
  });

  it("filtra pelo escopo pedido", async () => {
    render(<Probe scope="agent:meeting" />);
    await act(async () => {
      // Uma chamada de outra tarefa não deve aparecer.
      events.handlers[0]?.(usage("capability:summary", "gpt-4o"));
    });
    expect(screen.getByTestId("usage").textContent).toBe("nenhum");

    await act(async () => {
      events.handlers[0]?.(usage("agent:meeting", "deepseek-reasoner"));
    });
    expect(screen.getByTestId("usage").textContent).toBe(
      "agent:meeting|deepseek-reasoner",
    );
  });

  it("sem escopo, mostra qualquer tarefa", async () => {
    render(<Probe />);
    await act(async () => {
      events.handlers[0]?.(usage("capability:summary", "gpt-4o"));
    });
    expect(screen.getByTestId("usage").textContent).toBe(
      "capability:summary|gpt-4o",
    );
  });
});
