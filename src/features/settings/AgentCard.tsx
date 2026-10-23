/**
 * Card de um agente na aba "Assistentes" (BR-074).
 *
 * Mostra a identidade (ícone na cor do agente — BR-037), a badge "Ativo"
 * (informativa: quem manda é o seletor global do notch — BR-080) e a leitura
 * dos dois prompts do agente: o da sugestão (usado hoje) e o do resumo
 * (BR-076).
 */

import type { CSSProperties, ReactNode } from "react";
import {
  agentColorToken,
  agentIconId,
  agentSummaryPrompt,
  type AssistantMode,
} from "../../core/domain/assistant";
import { AgentIcon } from "../../components/AgentIcon";
import { IconButton } from "../../components/IconButton";
import { InfoTooltip } from "../../components/InfoTooltip";
import { Pill } from "../../components/Pill";
import { useTranslation, type TranslationKey } from "../../core/l10n";
import {
  BoltIcon,
  CopyIcon,
  PencilIcon,
  SummaryIcon,
  TrashIcon,
} from "../../components/icons";

export interface AgentCardProps {
  mode: AssistantMode;
  /**
   * Agente **Padrão** — fonte do fallback quando um prompt do agente está
   * vazio (BR-079).
   */
  defaultMode?: AssistantMode | null;
  /** `true` quando é o agente selecionado nas sugestões (badge "Ativo"). */
  active: boolean;
  onEdit: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
}

interface PromptPanelProps {
  icon: ReactNode;
  titleKey: TranslationKey;
  text: string;
  /** Chave exibida quando nem o agente nem o Padrão têm o prompt. */
  emptyKey: TranslationKey;
  /** `true` quando o prompt exibido veio do agente Padrão (BR-079). */
  inherited?: boolean;
  /** Explicação do painel (tooltip — BR-081). */
  hint: string;
}

/** Painel de leitura de um prompt (texto truncado pelo CSS). */
function PromptPanel({
  icon,
  titleKey,
  text,
  emptyKey,
  inherited,
  hint,
}: PromptPanelProps) {
  const { t } = useTranslation();
  const title = t(titleKey);
  return (
    <section className="prompt-panel" aria-label={title}>
      <header className="prompt-panel__head">
        <span className="prompt-panel__title">
          {icon}
          <span>{title}</span>
        </span>
        <InfoTooltip className="prompt-panel__help" label={title} text={hint} />
      </header>
      <p
        className={
          text
            ? "prompt-panel__text"
            : "prompt-panel__text prompt-panel__text--empty"
        }
        title={text || undefined}
      >
        {text || t(emptyKey)}
      </p>
      {inherited ? (
        <span className="prompt-panel__inherited">
          {t("agent_card_inherited_default_agent")}
        </span>
      ) : null}
    </section>
  );
}

/** Um agente configurado. */
export function AgentCard({
  mode,
  defaultMode,
  active,
  onEdit,
  onDuplicate,
  onDelete,
}: AgentCardProps) {
  const { t } = useTranslation();
  const style = {
    "--agent-color": `var(--oh-agent-${agentColorToken(mode)})`,
  } as CSSProperties;

  // BR-079: prompt vazio (trim) usa o do agente Padrão.
  const ownSuggestion = mode.suggestion_prompt.trim();
  const ownSummary = agentSummaryPrompt(mode);
  const fallbackSuggestion = defaultMode?.suggestion_prompt.trim() ?? "";
  const fallbackSummary = agentSummaryPrompt(defaultMode);
  const suggestion = ownSuggestion || fallbackSuggestion;
  const summary = ownSummary || fallbackSummary;
  const suggestionInherited = !ownSuggestion && Boolean(fallbackSuggestion);
  const summaryInherited = !ownSummary && Boolean(fallbackSummary);

  return (
    <article
      className={active ? "agent-card agent-card--active" : "agent-card"}
      style={style}
      aria-label={t("agent_card_agent_name", { name: mode.name })}
    >
      <header className="agent-card__head">
        <span className="agent-card__tile" aria-hidden="true">
          <AgentIcon icon={agentIconId(mode)} size={18} />
        </span>
        <div className="agent-card__identity">
          <h3 className="agent-card__name">{mode.name}</h3>
          {active ? (
            <Pill tone="accent" mono>
              {t("agent_card_active")}
            </Pill>
          ) : null}
        </div>
        <div className="agent-card__actions">
          <button
            type="button"
            className="settings__button settings__button--small agent-card__edit"
            aria-label={t("agent_card_edit_agent_name", { name: mode.name })}
            onClick={onEdit}
          >
            <PencilIcon size={13} />
            <span>{t("agent_card_edit")}</span>
          </button>
          <IconButton
            label={t("agent_card_duplicate_agent_name", { name: mode.name })}
            title={t("agent_card_duplicate")}
            onClick={onDuplicate}
          >
            <CopyIcon size={15} />
          </IconButton>
          <IconButton
            label={t("agent_card_delete_agent_name", { name: mode.name })}
            title={t("agent_card_delete")}
            onClick={onDelete}
          >
            <TrashIcon size={15} />
          </IconButton>
        </div>
      </header>

      <div className="agent-card__prompts">
        <PromptPanel
          icon={<BoltIcon size={13} />}
          titleKey="agent_editor_suggestion_prompt"
          text={suggestion}
          emptyKey="agent_editor_no_suggestion_prompt"
          inherited={suggestionInherited}
          hint={t("agent_card_used_today_basis_live_suggestion_system")}
        />
        <PromptPanel
          icon={<SummaryIcon size={13} />}
          titleKey="agent_editor_summary_prompt"
          text={summary}
          emptyKey="agent_editor_no_summary_prompt"
          inherited={summaryInherited}
          hint={t("agent_card_used_summary_agent_active_empty_inherits")}
        />
      </div>
    </article>
  );
}
