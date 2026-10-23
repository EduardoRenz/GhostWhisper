/**
 * Seletor **global** de agentes, flutuando acima da barra do notch (BR-080).
 *
 * O pill abre o menu em **janela própria** (`MenuWindow`, BR-072/ISS-063): a
 * lista e a escolha vivem lá. Assim o notch não muda de tamanho nem de posição
 * ao abrir/fechar o menu.
 *
 * Só o que tem `data-interactive` captura o mouse no overlay; o resto continua
 * click-through.
 */

import { useRef, type CSSProperties } from "react";
import {
  DEFAULT_AGENT_COLOR,
  DEFAULT_AGENT_ICON,
  PROTECTED_MODE_ID,
  agentColorToken,
  agentIconId,
  type AssistantMode,
} from "../../core/domain/assistant";
import { AgentIcon } from "../../components/AgentIcon";
import { ChevronDownIcon } from "../../components/icons";
import { useTranslation } from "../../core/l10n";

export interface AgentSwitcherProps {
  /** Catálogo efetivo (protegido + lista do usuário). */
  modes: AssistantMode[];
  /** Agente ativo (`settings.assistant_mode_id`). */
  modeId: string;
  /** Abre o menu flutuante ancorado no pill (CSS px relativo à janela). */
  onOpenMenu?: (anchor: DOMRect) => void;
}

/** Pill do seletor de agente do notch. */
export function AgentSwitcher({
  modes,
  modeId,
  onOpenMenu,
}: AgentSwitcherProps) {
  const { t } = useTranslation();
  const buttonRef = useRef<HTMLButtonElement>(null);

  const selected = modes.find((mode) => mode.id === modeId) ?? modes[0];
  // Fallback fora do catálogo (boot/janela do navegador): o Padrão é sempre
  // conhecido pelas constantes do domínio.
  const name =
    selected?.name ??
    (modeId === PROTECTED_MODE_ID
      ? t("assistants_default_agent")
      : t("agent_switcher_agent"));
  const icon = selected ? agentIconId(selected) : DEFAULT_AGENT_ICON;
  const color = selected ? agentColorToken(selected) : DEFAULT_AGENT_COLOR;

  const style = {
    "--agent-color": `var(--oh-agent-${color})`,
  } as CSSProperties;

  return (
    <div className="agent-switcher" style={style} data-interactive>
      <button
        ref={buttonRef}
        type="button"
        className="agent-switcher__button"
        aria-haspopup="menu"
        aria-label={t("agent_switcher_agent_name", { name })}
        title={t("agent_switcher_agent_name", { name })}
        onClick={() => {
          const rect = buttonRef.current?.getBoundingClientRect();
          if (rect) {
            onOpenMenu?.(rect);
          }
        }}
      >
        <span className="agent-switcher__icon">
          <AgentIcon icon={icon} size={14} />
        </span>
        <span className="agent-switcher__name">{name}</span>
        <span className="agent-switcher__divider" aria-hidden="true" />
        <span className="agent-switcher__hint">
          {t("agent_switcher_switch_agent")}
        </span>
        <ChevronDownIcon size={13} />
      </button>
    </div>
  );
}
