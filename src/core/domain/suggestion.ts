/**
 * Domínio da sugestão: título derivativo e metadados do Markdown.
 *
 * O **título** reusa o padrão que o resumo já usa (`subject` → primeiro heading
 * → rótulo neutro): é dinâmico, vem do texto e não exige campo novo no modelo
 * (BR-038). Os **metadados** são as linhas que os agentes técnicos já pedem
 * (`**Big O:** …`, `**Language:** …`) mais o nome do arquivo, e alimentam o
 * cabeçalho/rodapé dos blocos de código.
 */

/** Rótulo neutro quando o texto não tem título nenhum. */
export const NO_SUGGESTION_TITLE = "Sugestão";

/** Limite do título (curto, como no cartão). */
export const MAX_TITLE_CHARS = 72;

/** Remove a marcação do Markdown do começo de uma linha. */
function stripMarkdown(line: string): string {
  return line
    .replace(/^#{1,6}\s+/, "")
    .replace(/^>\s?/, "")
    .replace(/^[-*+]\s+/, "")
    .replace(/^\d+\.\s+/, "")
    .replace(/[*_`]/g, "")
    .trim();
}

/** Seções genéricas do contrato dos agentes (não servem de título). */
const GENERIC_SECTIONS = new Set([
  "easy",
  "practical",
  "deep",
  "glossary",
  "glossario",
  "overview",
  "visao geral",
  "what's being said",
  "whats being said",
  "o que esta sendo dito",
  "actions",
  "acoes",
  "initial implementation",
  "implementacao inicial",
  "approach",
  "abordagem",
  "complexity",
  "complexidade",
  "tests",
  "testes",
  "optimizations",
  "otimizacoes",
  "solution",
  "solucao",
  "code",
  "codigo",
  "language detected",
  "big o",
  "requirements",
  "requisitos",
  "estimates",
  "estimativas",
  "design",
  "summary",
  "resumo",
  "context",
  "contexto",
  "example",
  "exemplo",
  "notes",
  "notas",
]);

/** Normaliza para comparar com as seções genéricas (sem acento/pontuação). */
function normalize(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[.:!?]+$/g, "")
    .trim();
}

/** `true` quando a linha é uma seção/label genérica (ex.: “Easy”). */
function isGenericSection(value: string): boolean {
  const normalized = normalize(value);
  if (!normalized) {
    return true;
  }
  if (GENERIC_SECTIONS.has(normalized)) {
    return true;
  }
  // “Big O: O(n) time…”, “Language detected: python” etc.
  return (
    normalized.startsWith("big o") ||
    normalized.startsWith("language detected") ||
    normalized.startsWith("implementacao inicial") ||
    normalized.startsWith("initial implementation")
  );
}

/**
 * Título da sugestão: `Subject:` do modelo → primeira linha significativa
 * (heading ou conteúdo) → rótulo neutro.
 *
 * Ignora blocos de código, metadados (`**Big O:**`, `**File:**`), linhas
 * inteiras em negrito e **seções genéricas** do contrato dos agentes
 * (`## Easy`, `## Practical`, `**Initial implementation**`…): sem isso, uma
 * série de perguntas diferentes virava “Easy”, “Easy”, “Easy”.
 *
 * O título é **curto**: corta na palavra, com reticências.
 */
export function suggestionTitle(
  text: string | null | undefined,
  maxChars = MAX_TITLE_CHARS,
): string {
  const source = (text ?? "").trim();
  if (!source) {
    return NO_SUGGESTION_TITLE;
  }
  const subject = /^subject:\s*(.+)$/im.exec(source)?.[1]?.trim();
  if (subject) {
    return truncate(stripMarkdown(subject), maxChars);
  }

  let inFence = false;
  for (const raw of source.split("\n")) {
    const line = raw.trim();
    if (line.startsWith("```")) {
      inFence = !inFence;
      continue;
    }
    if (inFence || !line) {
      continue;
    }
    // Tabela, metadados e linha inteira em negrito são rótulos, não título.
    if (line.startsWith("|") || /^\*\*[^*]+:?\*\*:?$/.test(line)) {
      continue;
    }
    if (/^\*\*[^*]+:\*\*/.test(line)) {
      continue;
    }
    const heading = /^#{1,6}\s+(.+)$/.exec(line)?.[1];
    const value = stripMarkdown(heading ?? line);
    if (!value || isGenericSection(value)) {
      continue;
    }
    return truncate(value, maxChars);
  }

  return NO_SUGGESTION_TITLE;
}

/** Corta o texto no limite, na palavra, com reticências. */
function truncate(text: string, maxChars: number): string {
  if (text.length <= maxChars) {
    return text;
  }
  const cut = text.slice(0, maxChars);
  const lastSpace = cut.lastIndexOf(" ");
  return `${(lastSpace > 20 ? cut.slice(0, lastSpace) : cut).trimEnd()}…`;
}

/** Metadados que os agentes técnicos emitem no Markdown. */
export interface SuggestionMeta {
  /** Complexidade: “O(n) time | O(1) space”. */
  bigO?: string;
  /** Nome do arquivo do código (“solution.rs”). */
  file?: string;
}

/** Lê a linha `**Big O:** …` (o formato que os agentes já pedem). */
function parseBigO(markdown: string): string | undefined {
  const raw =
    /\*\*Big\s?O:\*\*\s*(.+)$/im.exec(markdown)?.[1]?.trim() ??
    /^Big\s?O:\s*(.+)$/im.exec(markdown)?.[1]?.trim();
  if (!raw) {
    return undefined;
  }
  return raw.replace(/\s*\|\s*/g, " · ");
}

/** Lê a linha `**File:** …` (o nome do arquivo do código). */
function parseFile(markdown: string): string | undefined {
  const raw =
    /\*\*File:\*\*\s*(.+)$/im.exec(markdown)?.[1]?.trim() ??
    /\*\*Arquivo:\*\*\s*(.+)$/im.exec(markdown)?.[1]?.trim();
  return raw && raw.length > 0 ? raw : undefined;
}

/** Extrai os metadados de uma sugestão em Markdown. */
export function parseSuggestionMeta(
  markdown: string | null | undefined,
): SuggestionMeta {
  const source = markdown ?? "";
  const bigO = parseBigO(source);
  const file = parseFile(source);
  return {
    ...(bigO ? { bigO } : {}),
    ...(file ? { file } : {}),
  };
}

/**
 * Remove as linhas de metadados do corpo do Markdown.
 *
 * Quando há bloco de código, o `Big O` e o arquivo aparecem no **caption** do
 * código (BR-038); repetir as linhas no corpo é ruído.
 */
export function stripSuggestionMeta(markdown: string): string {
  return markdown
    .split("\n")
    .filter((line) => !/^\s*\*\*(Big\s?O|Language|File|Arquivo):\*\*/i.test(line))
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

/** Tipo do bloco completo, pelo conteúdo (chip de metadados do cartão). */
export function suggestionKind(
  markdown: string | null | undefined,
): "Código" | "Explicativo" {
  return /```/.test(markdown ?? "") ? "Código" : "Explicativo";
}
