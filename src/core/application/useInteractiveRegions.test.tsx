import { render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useInteractiveRegions } from "./useInteractiveRegions";

const ipc = vi.hoisted(() => ({
  invokeSafe: vi.fn((_command: string, _args?: unknown) =>
    Promise.resolve(undefined as unknown),
  ),
}));

vi.mock("../../adapters/ipc/client", () => ({
  invokeSafe: ipc.invokeSafe,
}));

/** Componente mínimo com uma região interativa e a barra de nível. */
function Harness() {
  useInteractiveRegions();
  return (
    <div>
      <div className="levels-group" data-interactive>
        <div className="levels__fill" id="fill" style={{ width: "0%" }} />
      </div>
      <button type="button" id="action" data-interactive>
        Ação
      </button>
    </div>
  );
}

const publishes = (): number => ipc.invokeSafe.mock.calls.length;

/** Espera o próximo frame de animação (o hook publica com debounce de rAF). */
const nextFrame = () => new Promise((resolve) => setTimeout(resolve, 40));

describe("useInteractiveRegions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("publica as regiões interativas ao montar", async () => {
    render(<Harness />);

    await waitFor(() => {
      expect(publishes()).toBeGreaterThan(0);
    });

    const [command, payload] = ipc.invokeSafe.mock.calls[0]!;
    expect(command).toBe("overlay_set_interactive_regions");
    expect((payload as { regions: unknown[] }).regions).toHaveLength(2);
  });

  it("não republica quando só a barra de nível muda (ISS-020)", async () => {
    render(<Harness />);
    await waitFor(() => expect(publishes()).toBeGreaterThan(0));
    ipc.invokeSafe.mockClear();

    // A barra de áudio muda de largura ~10×/s por canal: nada clicável se mexeu.
    const fill = document.getElementById("fill") as HTMLElement;
    for (const width of ["10%", "35%", "70%", "20%"]) {
      fill.style.width = width;
      await nextFrame();
    }

    expect(publishes()).toBe(0);
  });

  it("republica quando uma região interativa muda", async () => {
    render(<Harness />);
    await waitFor(() => expect(publishes()).toBeGreaterThan(0));
    ipc.invokeSafe.mockClear();

    const action = document.getElementById("action") as HTMLElement;
    action.className = "outro-estilo";

    await waitFor(() => expect(publishes()).toBeGreaterThan(0));
  });
});
