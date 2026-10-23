/**
 * Guarda de regressão da escala de fonte (BR-087).
 *
 * Três invariantes que quebram o recurso se alguém reescrever o CSS:
 *
 * 1. Os tokens de tipografia precisam estar em `rem` — em `px` o zoom da raiz
 *    (`html { font-size: calc(base × escala) }`) não alcançaria fonte nem
 *    entrelinha e a UI inteira ficaria estática.
 * 2. Nenhum `font-size` fixo em `px` no `globals.css` (mesma razão).
 * 3. As superfícies de IA precisam multiplicar por `--oh-ai-font-scale` e o
 *    override precisa vir **depois** da regra original (a ordem decide a
 *    especificidade empate).
 *
 * Também trava o detalhe que causaria estouro: alturas fixas em `px` de
 * controles com texto passam a crescer junto (em `rem`).
 */

import { beforeAll, describe, expect, it } from "vitest";

let globals = "";
let tokens = "";

beforeAll(async () => {
  // O tsconfig do frontend não inclui os tipos de Node; o teste roda no
  // Vitest (Node), onde `node:fs` existe. Caminho relativo à raiz do projeto.
  // @ts-ignore
  const { readFileSync } = await import("node:fs");
  globals = readFileSync("src/styles/globals.css", "utf8");
  tokens = readFileSync("src/styles/tokens.css", "utf8");
});

/** Todos os corpos de regra de um seletor, na ordem do arquivo. */
function ruleBodies(css: string, selector: string): string[] {
  const bodies: string[] = [];
  let from = 0;
  for (;;) {
    const start = css.indexOf(`${selector} {`, from);
    if (start < 0) break;
    const open = css.indexOf("{", start);
    const close = css.indexOf("}", open);
    bodies.push(css.slice(open + 1, close));
    from = close + 1;
  }
  if (bodies.length === 0) {
    throw new Error(`regra não encontrada: ${selector}`);
  }
  return bodies;
}

/** Último corpo de regra de um seletor (o que vence no CSS). */
function lastRuleBody(css: string, selector: string): string {
  const bodies = ruleBodies(css, selector);
  const last = bodies[bodies.length - 1];
  if (last === undefined) {
    throw new Error(`regra vazia: ${selector}`);
  }
  return last;
}

describe("escala de fonte (BR-087)", () => {
  it("os tokens de tipografia estão em rem (o zoom da raiz alcança)", () => {
    const typography = tokens.match(
      /--oh-(?:headline|body|label|code|status)[a-z0-9-]*-(?:size|line):\s*[^;]+;/g,
    );
    expect(typography).not.toBeNull();
    expect(typography!.length).toBeGreaterThanOrEqual(19);
    for (const declaration of typography!) {
      expect(declaration, `${declaration} deve estar em rem`).toMatch(
        /:\s*\d+(\.\d+)?rem;/,
      );
    }
  });

  it("declara o multiplicador de texto de IA com o default 1", () => {
    expect(tokens).toContain("--oh-ai-font-scale: 1");
  });

  it("nenhum font-size fixo em px sobrou no globals.css", () => {
    expect(globals.match(/font-size:\s*\d+px/g)).toBeNull();
  });

  it("controles com texto crescem junto (altura em rem, não em px)", () => {
    expect(lastRuleBody(globals, ".session-pill")).toContain(
      "height: 1.75rem",
    );
    expect(lastRuleBody(globals, ".composer__input")).toContain(
      "height: 2rem",
    );
    expect(lastRuleBody(globals, ".composer__remove")).toContain(
      "width: 1.125rem",
    );
  });

  it("toda superfície de IA multiplica pelo bônus --oh-ai-font-scale", () => {
    const surfaces = [
      ".chat__row--ai .chat__text",
      ".chat__translation",
      ".markdown",
      ".markdown--compact",
      ".summary-view__subject",
      ".suggestion-card__title",
      ".suggestion-block__text",
      ".history__summary",
      ".insights__text",
    ];
    for (const selector of surfaces) {
      const body = lastRuleBody(globals, selector);
      expect(body, `${selector} sem bônus de IA`).toContain(
        "var(--oh-ai-font-scale)",
      );
    }
  });

  it("o override de IA vem depois da regra original (a ordem decide)", () => {
    const bodies = ruleBodies(globals, ".markdown");
    expect(bodies[bodies.length - 1]).toContain("var(--oh-ai-font-scale)");
    expect(bodies[0]).not.toContain("var(--oh-ai-font-scale)");

    const titles = ruleBodies(globals, ".suggestion-card__title");
    expect(titles[titles.length - 1]).toContain("var(--oh-ai-font-scale)");
  });
});
