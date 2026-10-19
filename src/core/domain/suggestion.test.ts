import { describe, expect, it } from "vitest";
import {
  NO_SUGGESTION_TITLE,
  parseSuggestionMeta,
  suggestionKind,
  suggestionTitle,
} from "./suggestion";

describe("suggestionTitle", () => {
  it("prefere o subject do modelo", () => {
    expect(suggestionTitle("Subject: Abordagem com Sliding Window\n\nresto")).toBe(
      "Abordagem com Sliding Window",
    );
  });

  it("usa o primeiro heading quando não há subject", () => {
    expect(suggestionTitle("## Estratégia Sliding Window\n\ntexto")).toBe(
      "Estratégia Sliding Window",
    );
  });

  it("usa a primeira linha significativa sem subject nem heading", () => {
    expect(suggestionTitle("> Proponho a abordagem com dois ponteiros")).toBe(
      "Proponho a abordagem com dois ponteiros",
    );
  });

  it("ignora as linhas de metadados", () => {
    expect(suggestionTitle("**Big O:** O(n) time | O(1) space")).toBe(
      NO_SUGGESTION_TITLE,
    );
  });

  it("pula seções genéricas e usa a primeira linha de conteúdo", () => {
    // Sem isso, uma série de perguntas diferentes virava “Easy”, “Easy”, …
    expect(
      suggestionTitle("## Easy\n\nEu inverteria a árvore com recursão."),
    ).toBe("Eu inverteria a árvore com recursão.");
    expect(
      suggestionTitle("## Practical\n\nDá para resolver com dois ponteiros."),
    ).toBe("Dá para resolver com dois ponteiros.");
    // “Glossary” é genérico, mas o conteúdo seguinte serve.
    expect(suggestionTitle("## Glossary\n\nHashMap: …")).toBe("HashMap: …");
  });

  it("pula rótulos em negrito e blocos de código", () => {
    expect(
      suggestionTitle(
        "**Initial implementation**\n\n```rust\npub fn main() {}\n```",
      ),
    ).toBe(NO_SUGGESTION_TITLE);
    expect(
      suggestionTitle(
        "**Big O:** O(n) time\n**Language:** rust\n**File:** solution.rs\n\nEstratégia Sliding Window: mantemos dois ponteiros.",
      ),
    ).toBe("Estratégia Sliding Window: mantemos dois ponteiros.");
  });

  it("usa um rótulo neutro sem texto", () => {
    expect(suggestionTitle("")).toBe(NO_SUGGESTION_TITLE);
    expect(suggestionTitle(null)).toBe(NO_SUGGESTION_TITLE);
  });
});

describe("parseSuggestionMeta", () => {
  it("lê o Big O e o arquivo", () => {
    const meta = parseSuggestionMeta(
      "**Big O:** O(n) time | O(1) space\n**Language:** rust\n**File:** solution.rs\n\n```rust\nfn main() {}\n```",
    );
    expect(meta.bigO).toBe("O(n) time · O(1) space");
    expect(meta.file).toBe("solution.rs");
  });

  it("aceita o formato sem asteriscos", () => {
    const meta = parseSuggestionMeta("Big O: O(n) time | O(1) space");
    expect(meta.bigO).toBe("O(n) time · O(1) space");
    expect(meta.file).toBeUndefined();
  });

  it("não inventa metadados ausentes", () => {
    expect(parseSuggestionMeta("só um texto")).toEqual({});
    expect(parseSuggestionMeta(null)).toEqual({});
  });
});

describe("suggestionKind", () => {
  it("distingue código de texto explicativo", () => {
    expect(suggestionKind("```rust\nfn main() {}\n```")).toBe("Código");
    expect(suggestionKind("Estratégia Sliding Window")).toBe("Explicativo");
  });
});
