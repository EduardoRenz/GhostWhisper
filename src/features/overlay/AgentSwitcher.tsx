/**
 * Seletor **global** de agentes, flutuando acima da barra do notch (BR-080).
 *
 * É compacto e discreto: mostra o ícone (na cor do agente), o nome e o convite
 * "Alternar agente". O popover lista o catálogo efetivo (Padrão + agentes do
 * usuário) e grava a escolha nas settings — a mudança vale na hora em todas as
 * janelas (`settings_changed`).
 *
 * Só o que tem `data-interactive` captura o mouse no overlay; o resto continua
 * click-through.
 */

import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import {
  DEFAULT_AGENT_COLOR,
  DEFAULT_AGENT_ICON,
  PROTECTED_MODE_ID,
  PROTECTED_MODE_NAME,
  agentColorToken,
  agentIconId,
  type AssistantMode,
} from "../../core/domain/assistant";
import { AgentIcon } from "../../components/AgentIcon";
import { CheckIcon, ChevronDownIcon } from "../../components/icons";

export interface AgentSwitcherProps {
  /** Catálogo efetivo (protegido + lista do usuário). */
  modes: AssistantMode[];
  /** Agente ativo (`settings.assistant_mode_id`). */
  modeId: string;
  /** Troca o agente (persiste nas settings). */
  onSelect: (id: string) => void;
  /** Avisa o overlay para reservar altura enquanto o menu está aberto. */
  onOpenChange?: (open: boolean) => void;
}

/** Seletor de agente flutuante do notch. */
export function AgentSwitcher({
  modes,
  modeId,
  onSelect,
  onOpenChange,
}: AgentSwitcherProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const selected = modes.find((mode) => mode.id === modeId) ?? modes[0];
  // Fallback fora do catálogo (boot/janela do navegador): o Padrão é sempre
  // conhecido pelas constantes do domínio.
  const name =
    selected?.name ??
    (modeId === PROTECTED_MODE_ID ? PROTECTED_MODE_NAME : "Agente");
  const icon = selected ? agentIconId(selected) : DEFAULT_AGENT_ICON;
  const color = selected ? agentColorToken(selected) : DEFAULT_AGENT_COLOR;

  useEffect(() => {
    onOpenChange?.(open);
  }, [open, onOpenChange]);

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

  const style = {
    "--agent-color": `var(--oh-agent-${color})`,
  } as CSSProperties;

  return (
    <div className="agent-switcher" ref={rootRef} style={style} data-interactive>
      <button
        type="button"
        className="agent-switcher__button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`Agente: ${name}`}
        title={`Agente: ${name}`}
        onClick={() => setOpen((previous) => !previous)}
      >
        <span className="agent-switcher__icon">
          <AgentIcon icon={icon} size={14} />
        </span>
        <span className="agent-switcher__name">{name}</span>
        <span className="agent-switcher__divider" aria-hidden="true" />
        <span className="agent-switcher__hint">Alternar agente</span>
        <ChevronDownIcon size={13} />
      </button>

      {open && (
        <div
          className="agent-switcher__popover"
          role="menu"
          aria-label="Agentes"
          data-interactive
          onKeyDown={onMenuKeyDown}
        >
          {modes.length === 0 && (
            <span className="agent-switcher__empty">Nenhum agente</span>
          )}
          {modes.map((mode, index) => {
            const on = mode.id === selected?.id;
            const itemStyle = {
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
                className={`agent-switcher__item${
                  on ? " agent-switcher__item--on" : ""
                }`}
                style={itemStyle}
                onClick={() => choose(mode.id)}
              >
                <span className="agent-switcher__icon">
                  <AgentIcon icon={agentIconId(mode)} size={15} />
                </span>
                <span className="agent-switcher__item-name">{mode.name}</span>
                {on && <CheckIcon size={14} />}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
