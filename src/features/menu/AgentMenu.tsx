import { useEffect, useRef, type CSSProperties } from "react";
import { AgentIcon } from "../../components/AgentIcon";
import { CheckIcon } from "../../components/icons";
import { setAssistantMode } from "../../adapters/ipc/settings";
import { closeMenu } from "../../adapters/ipc/overlay";
import { useAssistantModes } from "../../core/application/useAssistantModes";
import { useSettings } from "../../core/application/useSettings";
import {
  agentColorToken,
  agentIconId,
} from "../../core/domain/assistant";
import { useTranslation } from "../../core/l10n";

interface AgentMenuProps {
  /** Muda a cada abertura para recarregar o catálogo. */
  refreshKey?: number;
}

/** Lista de agentes dentro da janela flutuante (BR-072/ISS-063). */
export function AgentMenu({ refreshKey }: AgentMenuProps) {
  const { t } = useTranslation();
  const { modes } = useAssistantModes(refreshKey);
  const { settings } = useSettings();
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const selected = settings.assistant_mode_id;

  // Foco no agente ativo (ou no primeiro) ao abrir; o webview tem foco.
  useEffect(() => {
    const index = modes.findIndex((mode) => mode.id === selected);
    itemRefs.current[Math.max(index, 0)]?.focus();
  }, [modes, selected, refreshKey]);

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
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

  return (
    <div
      className="menu-list"
      role="menu"
      aria-label={t("agent_switcher_agents")}
      onKeyDown={onKeyDown}
    >
      {modes.length === 0 && (
        <span className="agent-switcher__empty">
          {t("agent_switcher_no_agents")}
        </span>
      )}
      {modes.map((mode, index) => {
        const on = mode.id === selected;
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
            className={`agent-switcher__item${
              on ? " agent-switcher__item--on" : ""
            }`}
            style={style}
            onClick={() => {
              void setAssistantMode(mode.id).finally(() => void closeMenu());
            }}
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
  );
}
