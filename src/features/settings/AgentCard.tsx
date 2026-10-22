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
  title: string;
  text: string;
  /** Texto exibido quando nem o agente nem o Padrão têm o prompt. */
  empty: string;
  /** `true` quando o prompt exibido veio do agente Padrão (BR-079). */
  inherited?: boolean;
  /** Explicação do painel (tooltip — BR-081). */
  hint: string;
}

/** Painel de leitura de um prompt (texto truncado pelo CSS). */
function PromptPanel({
  icon,
  title,
  text,
  empty,
  inherited,
  hint,
}: PromptPanelProps) {
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
        {text || empty}
      </p>
      {inherited ? (
        <span className="prompt-panel__inherited">
          Herdado do agente Padrão
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
      aria-label={`Agente ${mode.name}`}
    >
      <header className="agent-card__head">
        <span className="agent-card__tile" aria-hidden="true">
          <AgentIcon icon={agentIconId(mode)} size={18} />
        </span>
        <div className="agent-card__identity">
          <h3 className="agent-card__name">{mode.name}</h3>
          {active ? (
            <Pill tone="accent" mono>
              Ativo
            </Pill>
          ) : null}
        </div>
        <div className="agent-card__actions">
          <button
            type="button"
            className="settings__button settings__button--small agent-card__edit"
            aria-label={`Editar agente ${mode.name}`}
            onClick={onEdit}
          >
            <PencilIcon size={13} />
            <span>Editar</span>
          </button>
          <IconButton
            label={`Duplicar agente ${mode.name}`}
            title="Duplicar"
            onClick={onDuplicate}
          >
            <CopyIcon size={15} />
          </IconButton>
          <IconButton
            label={`Excluir agente ${mode.name}`}
            title="Excluir"
            onClick={onDelete}
          >
            <TrashIcon size={15} />
          </IconButton>
        </div>
      </header>

      <div className="agent-card__prompts">
        <PromptPanel
          icon={<BoltIcon size={13} />}
          title="Prompt de Sugestão"
          text={suggestion}
          empty="Sem prompt de sugestão."
          inherited={suggestionInherited}
          hint="Usado hoje: é a base do prompt de sistema da sugestão ao vivo; o contrato comum de transcrição (ME/THEY, ASR) é aplicado por cima."
        />
        <PromptPanel
          icon={<SummaryIcon size={13} />}
          title="Prompt de Resumo"
          text={summary}
          empty="Sem prompt de resumo."
          inherited={summaryInherited}
          hint="Salvo por agente, mas ainda não consumido: o resumo continua com o contrato global."
        />
      </div>
    </article>
  );
}
