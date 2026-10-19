import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusPill } from "./StatusPill";

describe("StatusPill", () => {
  it("expõe o texto como status e aplica o tom", () => {
    const { container } = render(<StatusPill label="Gerando…" tone="ai" />);
    expect(screen.getByRole("status").textContent).toContain("Gerando…");
    expect(container.querySelector(".status-pill--ai")).not.toBeNull();
    expect(container.querySelector(".status-pill__spinner")).not.toBeNull();
  });

  it("aceita dica de ferramenta e tom alternativo", () => {
    const { container } = render(
      <StatusPill label="Atualizando" tone="accent" title="dica" />,
    );
    expect(screen.getByRole("status").getAttribute("title")).toBe("dica");
    expect(container.querySelector(".status-pill--accent")).not.toBeNull();
  });
});
