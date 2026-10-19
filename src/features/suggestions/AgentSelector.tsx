import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import {
  agentColorToken,
  agentIconId,
  type AssistantMode,
} from "../../core/domain/assistant";
import { AgentIcon } from "../../components/AgentIcon";
import { CheckIcon, ChevronDownIcon } from "../../components/icons";
import { Pill } from "../../components/Pill";

interface AgentSelectorProps {
  /** Agentes disponíveis (embutidos + personalizados). */
  modes: AssistantMode[];
  /** Agente selecionado. */
  modeId: string;
  /** Troca o agente (persiste nas settings). */
  onSelect: (id: string) => void;
}

/**
 * Seletor de agente: botão de **largura total** + popover (BR-037).
 *
 * O botão mostra o ícone (na cor do agente), o nome e a pill **ATIVO**; o
 * popover lista todos os agentes com ícone e cor. O teclado navega com as
 * setas, `Enter` escolhe e `Escape` fecha.
 */
export function AgentSelector({ modes, modeId, onSelect }: AgentSelectorProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const selected = modes.find((mode) => mode.id === modeId) ?? modes[0];

  // Fecha ao clicar fora ou apertar Escape.
  useEffect(() => {
    if (!open) {
      return;
    }
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  /** Navegação por teclado dentro do popover. */
  const onMenuKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const items = itemRefs.current.filter(
        (item): item is HTMLButtonElement => item !== null,
      );
      if (items.length === 0) {
        return;
      }
      const current = items.findIndex((item) => item === document.activeElement);
      const step = event.key === "ArrowDown" ? 1 : -1;
      const next = (current + step + items.length) % items.length;
      items[next]?.focus();
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      itemRefs.current[0]?.focus();
    }
    if (event.key === "End") {
      event.preventDefault();
      itemRefs.current[itemRefs.current.length - 1]?.focus();
    }
  };

  const choose = (id: string) => {
    onSelect(id);
    setOpen(false);
  };

  const selectedStyle = {
    "--agent-color": `var(--oh-agent-${agentColorToken(selected)})`,
  } as CSSProperties;

  return (
    <div className="agent-selector" ref={rootRef} data-interactive>
      <button
        type="button"
        className="agent-selector__button"
        style={selectedStyle}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`Agente: ${selected?.name ?? "—"}`}
        onClick={() => setOpen((previous) => !previous)}
      >
        <span className="agent-selector__icon">
          <AgentIcon icon={agentIconId(selected)} size={15} />
        </span>
        <span className="agent-selector__name">{selected?.name ?? "—"}</span>
        <Pill className="agent-selector__pill" tone="accent" mono>
          ATIVO
        </Pill>
        <span className="agent-selector__hint">Alternar agente</span>
        <ChevronDownIcon size={14} />
      </button>

      {open && (
        <div
          className="agent-selector__popover"
          role="menu"
          aria-label="Agentes"
          data-interactive
          onKeyDown={onMenuKeyDown}
        >
          {modes.length === 0 && (
            <span className="agent-selector__empty">Nenhum agente</span>
          )}
          {modes.map((mode, index) => {
            const on = mode.id === selected?.id;
            const style = {
              "--agent-color": `var(--oh-agent-${agentColorToken(mode)})`,
            } as CSSProperties;
            return (
              <button
                key={mode.id}
                type="button"
                role="menuitemradio"
                aria-checked={on}
                ref={(element) => {
                  itemRefs.current[index] = element;
                }}
                className={`agent-selector__item${
                  on ? " agent-selector__item--on" : ""
                }`}
                style={style}
                onClick={() => choose(mode.id)}
              >
                <span className="agent-selector__icon">
                  <AgentIcon icon={agentIconId(mode)} size={15} />
                </span>
                <span className="agent-selector__item-name">{mode.name}</span>
                {on && <CheckIcon size={14} />}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
