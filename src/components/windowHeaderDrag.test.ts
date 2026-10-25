import { describe, expect, it } from "vitest";
import { shouldStartHeaderDrag } from "./windowHeaderDrag";

function renderTarget(html: string): HTMLElement {
  const host = document.createElement("div");
  host.innerHTML = html;
  return host.firstElementChild as HTMLElement;
}

describe("shouldStartHeaderDrag", () => {
  it("libera o header e áreas inertes (título, subtítulo, espaços)", () => {
    const header = renderTarget(
      '<header><div class="text"><h2>Título</h2><span>Subtítulo</span></div></header>',
    );
    const title = header.querySelector("h2")!;

    expect(shouldStartHeaderDrag(header)).toBe(true);
    expect(shouldStartHeaderDrag(title)).toBe(true);
  });

  it("ignora controles dentro do header", () => {
    const header = renderTarget(`
      <header>
        <button type="button">Ação</button>
        <a href="#x">Link</a>
        <input />
        <select></select>
        <textarea></textarea>
        <label>Rótulo</label>
        <div role="button">Falso botão</div>
        <div role="tab">Aba</div>
        <div contenteditable="true">Edição</div>
      </header>
    `);

    for (const control of Array.from(header.children)) {
      expect(shouldStartHeaderDrag(control), control.outerHTML).toBe(false);
    }
  });

  it("ignora o conteúdo interno do controle (alvo é o ícone/texto)", () => {
    const header = renderTarget(
      '<header><button type="button"><span><svg><path /></svg></span></button></header>',
    );
    const path = header.querySelector("path")!;

    expect(shouldStartHeaderDrag(path)).toBe(false);
  });

  it("ignora a alça: ela tem handler próprio e não deve arrastar em dobro", () => {
    const header = renderTarget(
      '<header><span class="drag-handle"><svg><path /></svg></span></header>',
    );

    expect(shouldStartHeaderDrag(header.querySelector(".drag-handle"))).toBe(
      false,
    );
    expect(shouldStartHeaderDrag(header.querySelector("path"))).toBe(false);
  });

  it("respeita a escotilha data-no-window-drag", () => {
    const header = renderTarget(
      '<header><div data-no-window-drag><span>Conteúdo</span></div></header>',
    );
    const optOut = header.firstElementChild as HTMLElement;

    expect(shouldStartHeaderDrag(optOut)).toBe(false);
    expect(shouldStartHeaderDrag(optOut.querySelector("span"))).toBe(false);
  });

  it("não arrasta sem alvo", () => {
    expect(shouldStartHeaderDrag(null)).toBe(false);
  });
});
