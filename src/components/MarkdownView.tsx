import { isValidElement, memo, useCallback, useMemo, useState } from "react";
import type { ReactNode } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isTauriRuntime } from "../adapters/ipc/client";
import { copyText } from "../core/application/clipboard";
import { useTranslation } from "../core/l10n";
import {
  parseSuggestionMeta,
  type SuggestionMeta,
} from "../core/domain/suggestion";

/** Extrai o texto cru de uma subárvore React (para copiar o bloco). */
function extractText(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") {
    return String(node);
  }
  if (Array.isArray(node)) {
    return node.map(extractText).join("");
  }
  if (isValidElement(node)) {
    return extractText((node.props as { children?: ReactNode }).children);
  }
  return "";
}

/**
 * Bloco de código com cabeçalho (linguagem + arquivo + complexidade), botão
 * copiar e realce.
 *
 * Os metadados vêm das linhas que o modelo emite (`**Big O:**`, `**File:**`):
 * o cabeçalho mostra `linguagem — arquivo` e a complexidade à direita
 * (BR-038/task 05).
 */
function CodeBlock({
  language,
  code,
  meta,
  children,
}: {
  language: string;
  code: string;
  meta: SuggestionMeta;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  const onCopy = useCallback(async () => {
    if (await copyText(code)) {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1_500);
    }
  }, [code]);

  return (
    <div className="md-code">
      <div className="md-code__head">
        <span className="md-code__lang">
          {language}
          {meta.file && (
            <span className="md-code__file"> — {meta.file}</span>
          )}
        </span>
        {meta.bigO && <span className="md-code__meta">{meta.bigO}</span>}
        <button
          type="button"
          className="md-code__copy"
          onClick={() => void onCopy()}
        >
          {copied ? t("common_copied") : t("common_copy")}
        </button>
      </div>
      <pre className="md-code__pre">
        <code className={`hljs language-${language}`}>{children}</code>
      </pre>
    </div>
  );
}

/** Abre um link no navegador (Tauri) ou numa aba nova (dev no navegador). */
async function openExternal(href?: string): Promise<void> {
  if (!href) {
    return;
  }
  try {
    if (isTauriRuntime()) {
      await openUrl(href);
    } else {
      window.open(href, "_blank", "noopener,noreferrer");
    }
  } catch {
    // best-effort: link quebrado não derruba a janela.
  }
}

const components = (meta: SuggestionMeta): Components => ({
  // O `code` só embrulha; o `pre` decide se é bloco (com linguagem) ou inline.
  code: ({ className, children, ...props }) => (
    <code className={`md-inline ${className ?? ""}`.trim()} {...props}>
      {children}
    </code>
  ),
  pre: ({ children }) => {
    const child = Array.isArray(children) ? children[0] : children;
    if (!isValidElement(child)) {
      return <pre className="md-code__plain">{children}</pre>;
    }
    const codeProps = child.props as {
      className?: string;
      children?: ReactNode;
    };
    const language = /language-([\w-]+)/.exec(codeProps.className ?? "")?.[1];
    if (!language) {
      return (
        <pre className="md-code__plain">
          <code>{codeProps.children}</code>
        </pre>
      );
    }
    return (
      <CodeBlock
        language={language}
        code={extractText(codeProps.children).replace(/\n$/, "")}
        meta={meta}
      >
        {codeProps.children}
      </CodeBlock>
    );
  },
  a: ({ href, children }) => (
    <a
      href={href}
      onClick={(event) => {
        event.preventDefault();
        void openExternal(href);
      }}
    >
      {children}
    </a>
  ),
});

interface MarkdownViewProps {
  markdown: string;
  /** Versão compacta (bolhas do chat). */
  compact?: boolean;
  className?: string;
  /**
   * Metadados do código (arquivo, complexidade).
   *
   * Quando o corpo foi limpo das linhas de metadados, o chamador passa os
   * metadados já lidos — senão o caption do código fica sem informação.
   */
  meta?: SuggestionMeta;
}

/**
 * Renderer de Markdown do produto: GFM, código com realce e botão copiar,
 * links externos e **sem HTML bruto** (segurança).
 */
export const MarkdownView = memo(function MarkdownView({
  markdown,
  compact = false,
  className,
  meta: metaProp,
}: MarkdownViewProps) {
  // Metadados do código (linguagem, arquivo, complexidade) — BR-038.
  const parsed = useMemo(() => parseSuggestionMeta(markdown), [markdown]);
  const meta = metaProp ?? parsed;
  return (
    <div
      className={[
        "markdown",
        compact ? "markdown--compact" : "",
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[[rehypeHighlight, { detect: false, ignoreMissing: true }]]}
        components={components(meta)}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  );
});
