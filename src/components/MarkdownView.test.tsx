import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MarkdownView } from "./MarkdownView";

const copyText = vi.fn();

vi.mock("../core/application/clipboard", () => ({
  copyText: (...args: unknown[]) => copyText(...args),
}));

describe("MarkdownView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    copyText.mockResolvedValue(true);
  });

  it("renderiza títulos, listas, tabelas e código inline (GFM)", () => {
    const { container } = render(
      <MarkdownView
        markdown={[
          "# Título",
          "",
          "- um",
          "- dois",
          "",
          "| A | B |",
          "| - | - |",
          "| 1 | 2 |",
          "",
          "Use `cargo test`.",
        ].join("\n")}
      />,
    );

    expect(screen.getByRole("heading", { name: "Título" })).toBeDefined();
    expect(screen.getByText("um")).toBeDefined();
    expect(container.querySelector("table")).not.toBeNull();
    expect(container.querySelector("code.md-inline")?.textContent).toBe(
      "cargo test",
    );
  });

  it("renderiza bloco de código com linguagem e realce", () => {
    const { container } = render(
      <MarkdownView markdown={"```rust\nfn main() {}\n```"} />,
    );

    // O cabeçalho mostra a linguagem detectada.
    expect(screen.getByText("rust")).toBeDefined();
    const code = container.querySelector("code.hljs");
    expect(code).not.toBeNull();
    expect(code?.textContent).toContain("fn main");
  });

  it("não executa HTML bruto", () => {
    const { container } = render(
      <MarkdownView markdown={"<script>alert(1)</script>\n\n**ok**"} />,
    );

    expect(container.querySelector("script")).toBeNull();
    expect(screen.getByText("ok")).toBeDefined();
  });

  it("copia o bloco de código", async () => {
    render(<MarkdownView markdown={"```ts\nconst x = 1;\n```"} />);

    fireEvent.click(screen.getByRole("button", { name: "Copiar" }));

    await waitFor(() => {
      expect(copyText).toHaveBeenCalledWith("const x = 1;");
    });
    expect(await screen.findByText("Copiado")).toBeDefined();
  });

  it("abre links no navegador", () => {
    const open = vi.fn();
    vi.stubGlobal("open", open);

    render(<MarkdownView markdown={"[site](https://example.com)"} />);
    fireEvent.click(screen.getByRole("link", { name: "site" }));

    expect(open).toHaveBeenCalledWith(
      "https://example.com",
      "_blank",
      "noopener,noreferrer",
    );
    vi.unstubAllGlobals();
  });
});
