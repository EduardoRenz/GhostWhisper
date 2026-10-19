import { useEffect, useRef, useState } from "react";
import type { Suggestion } from "../../core/domain/assistant";
import { suggestionTitle } from "../../core/domain/suggestion";
import { AgentBadge } from "../../components/AgentBadge";
import { AgentIcon } from "../../components/AgentIcon";
import { CopyIcon, HistoryIcon, PinIcon } from "../../components/icons";
import { Pill } from "../../components/Pill";
import { MarkdownView } from "../../components/MarkdownView";

interface SuggestionHistoryProps {
  /** Sugestões anteriores (a atual fica no topo). */
  items: Suggestion[];
  /** Copia o texto de uma sugestão do histórico. */
  onCopy: (suggestion: Suggestion) => void;
  /** Fixa uma sugestão do histórico na conversa. */
  onPin: (suggestion: Suggestion) => void;
  /** Id da mensagem fixada (para o feedback). */
  pinnedId: string | null;
}

/** Chave estável de um item do histórico. */
const keyOf = (suggestion: Suggestion): string =>
  `${suggestion.at_ms}-${suggestion.message_id ?? suggestion.text.slice(0, 12)}`;

/**
 * Histórico da sessão: seção colapsável + accordions por sugestão (BR-038).
 *
 * Expandir um item abre o conteúdo **no lugar** e rola até ele; a sugestão
 * atual **permanece no topo** (nada é substituído — CA-04.2). Nenhum outro
 * componente fica por cima do bloco (CA-04.6).
 */
export function SuggestionHistory({
  items,
  onCopy,
  onPin,
  pinnedId,
}: SuggestionHistoryProps) {
  const [sectionOpen, setSectionOpen] = useState(true);
  const [expanded, setExpanded] = useState<string | null>(null);
  const expandedRef = useRef<HTMLDivElement>(null);

  // Ao expandir, o scroll vai até o bloco (sem trocar a seleção da janela).
  useEffect(() => {
    if (expanded && expandedRef.current) {
      // `scrollIntoView` pode não existir em webviews antigas: é feedback.
      expandedRef.current.scrollIntoView?.({ block: "nearest" });
    }
  }, [expanded]);

  if (items.length === 0) {
    return null;
  }

  return (
    <section className="history">
      <button
        type="button"
        className="history__head"
        aria-expanded={sectionOpen}
        onClick={() => setSectionOpen((previous) => !previous)}
      >
        <span className="history__icon">
          <HistoryIcon size={15} />
        </span>
        <span className="history__title">Histórico de Sugestões da Sessão</span>
        <Pill className="history__count">
          {items.length === 1 ? "1 anterior" : `${items.length} anteriores`}
        </Pill>
        <span className="history__chevron" aria-hidden="true">
          {sectionOpen ? "⌃" : "⌄"}
        </span>
      </button>

      {sectionOpen && (
        <ul className="history__list">
          {items.map((item) => {
            const key = keyOf(item);
            const open = expanded === key;
            return (
              <li key={key} className="history__item">
                <button
                  type="button"
                  className="history__item-head"
                  aria-expanded={open}
                  onClick={() => setExpanded(open ? null : key)}
                >
                  <span
                    className="history__item-icon"
                    style={{
                      color: `var(--oh-agent-${item.mode_color || "slate"})`,
                    }}
                    aria-hidden="true"
                  >
                    <AgentIcon icon={item.mode_icon} size={15} />
                  </span>
                  <span className="history__item-title">
                    {item.title?.trim() || suggestionTitle(item.text)}
                  </span>
                  <AgentBadge
                    mode={{
                      name: item.mode_name,
                      icon: item.mode_icon,
                      color: item.mode_color,
                    }}
                  />
                  <span className="history__chevron" aria-hidden="true">
                    {open ? "⌄" : "›"}
                  </span>
                </button>

                {open && (
                  <div className="history__item-body" ref={expandedRef}>
                    <MarkdownView markdown={item.text} />
                    <div className="history__item-actions">
                      <button
                        type="button"
                        className="icon-button"
                        onClick={() => onPin(item)}
                        title="Fixar na conversa"
                        aria-label="Fixar na conversa"
                      >
                        <PinIcon size={14} />
                      </button>
                      <button
                        type="button"
                        className="icon-button"
                        onClick={() => onCopy(item)}
                        title="Copiar"
                        aria-label="Copiar"
                      >
                        <CopyIcon size={14} />
                      </button>
                      {pinnedId && pinnedId === item.message_id && (
                        <span className="history__pinned">Fixada</span>
                      )}
                    </div>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
