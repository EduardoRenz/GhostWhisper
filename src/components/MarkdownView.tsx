import { isValidElement, memo, useCallback, useState } from "react";
import type { ReactNode } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { openUrl } from "@tauri-apps/plugin-opener";
import { isTauriRuntime } from "../adapters/ipc/client";
import { copyText } from "../core/application/clipboard";

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

/** Bloco de código com cabeçalho (linguagem + copiar) e realce. */
function CodeBlock({
  language,
  code,
  children,
}: {
  language: string;
  code: string;
  children: ReactNode;
}) {
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
        <span className="md-code__lang">{language}</span>
        <button
          type="button"
          className="md-code__copy"
          onClick={() => void onCopy()}
        >
          {copied ? "Copiado" : "Copiar"}
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

const components: Components = {
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
};

interface MarkdownViewProps {
  markdown: string;
  /** Versão compacta (bolhas do chat). */
  compact?: boolean;
  className?: string;
}

/**
 * Renderer de Markdown do produto: GFM, código com realce e botão copiar,
 * links externos e **sem HTML bruto** (segurança).
 */
export const MarkdownView = memo(function MarkdownView({
  markdown,
  compact = false,
  className,
}: MarkdownViewProps) {
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
        components={components}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  );
});
