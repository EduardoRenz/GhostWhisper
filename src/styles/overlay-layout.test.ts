/**
 * Guarda de regressão do layout do overlay (ISS-061).
 *
 * A largura do notch é **medida do conteúdo** e a janela acompanha. Um
 * `max-width: 100%` na pilha prendia a medição à largura atual da janela:
 * quando a barra crescia (início de sessão, menu), o conteúdo era clampado e
 * a janela nunca crescia — a barra ficava escondida atrás de uma barra de
 * rolagem. Este teste falharia no CSS anterior.
 */

import { beforeAll, describe, expect, it } from "vitest";

let css = "";

beforeAll(async () => {
  // O tsconfig do frontend não inclui os tipos de Node; o teste roda no
  // Vitest (Node), onde `node:fs` existe. O caminho é relativo à raiz do
  // projeto (cwd do `pnpm test`).
  // @ts-ignore
  const { readFileSync } = await import("node:fs");
  css = readFileSync("src/styles/globals.css", "utf8");
});

/** Corpo da primeira regra cujo seletor começa com `selector`. */
function ruleBody(selector: string): string {
  const start = css.indexOf(`${selector} {`);
  if (start < 0) {
    throw new Error(`regra não encontrada: ${selector}`);
  }
  const open = css.indexOf("{", start);
  const close = css.indexOf("}", open);
  return css.slice(open + 1, close);
}

describe("layout do overlay (ISS-061)", () => {
  it("a pilha do notch cresce com o conteúdo (sem max-width)", () => {
    const body = ruleBody(".overlay__stack");
    expect(body).toContain("width: max-content");
    expect(body).not.toContain("max-width");
  });

  it("a janela do overlay não rola no documento", () => {
    expect(css).toContain(".overlay-window,");
    expect(css).toContain(".overlay-window #root");

    const start = css.indexOf(".overlay-window,");
    const open = css.indexOf("{", start);
    const close = css.indexOf("}", open);
    expect(css.slice(open + 1, close)).toContain("overflow: hidden");
  });
});
