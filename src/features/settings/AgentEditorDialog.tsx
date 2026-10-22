/**
 * Modal de agente — criar, editar e duplicar (BR-077).
 *
 * O **mesmo** formulário serve aos três casos: os campos são os do mockup
 * (nome, ícone, cor, prompt de sugestão e prompt de resumo) e os que já
 * existiam no modelo (categoria, prompt de sistema e contexto) ficam no bloco
 * "avançado". Valida as mesmas regras do core: nome obrigatório e ao menos um
 * prompt preenchido.
 */

import { useState } from "react";
import {
  AGENT_COLOR_TOKENS,
  AGENT_ICON_IDS,
  DEFAULT_AGENT_COLOR,
  DEFAULT_AGENT_ICON,
  MODE_NAME_MAX_CHARS,
  MODE_PROMPT_MAX_CHARS,
  type AssistantMode,
} from "../../core/domain/assistant";
import { AgentIcon, AGENT_ICON_LABELS } from "../../components/AgentIcon";
import { Dialog } from "../../components/Dialog";
import { InfoTooltip } from "../../components/InfoTooltip";

export interface AgentEditorDialogProps {
  open: boolean;
  /** `edit` mantém o id; `create` usa o id do rascunho (novo ou duplicado). */
  kind: "create" | "edit";
  /** Agente de partida (valores atuais na edição, cópia na duplicação). */
  initial: AssistantMode | null;
  onCancel: () => void;
  onSave: (mode: AssistantMode) => void;
}

/** Formulário vazio de um agente novo. */
export function blankAgent(): AssistantMode {
  return {
    id: newAgentId(),
    name: "",
    category: "custom",
    system_prompt: "",
    suggestion_prompt: "",
    summary_prompt: "",
    context: "",
    icon: DEFAULT_AGENT_ICON,
    color: DEFAULT_AGENT_COLOR,
  };
}

/** Id estável para um agente criado pelo usuário. */
export function newAgentId(): string {
  const random =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.round(Math.random() * 1e6)}`;
  return `custom-${random}`;
}

/** `true` quando o agente tem ao menos um prompt/contexto (regra do core). */
export function hasAnyPrompt(mode: AssistantMode): boolean {
  return Boolean(
    mode.system_prompt.trim() ||
      mode.suggestion_prompt.trim() ||
      mode.summary_prompt.trim() ||
      mode.context.trim(),
  );
}

interface HelpLabelProps {
  htmlFor: string;
  label: string;
  /** Explicação do campo (tooltip — BR-081). */
  hint: string;
}

/** Rótulo do campo com o ícone de ajuda. */
function HelpLabel({ htmlFor, label, hint }: HelpLabelProps) {
  return (
    <div className="settings__field-label">
      <label htmlFor={htmlFor}>{label}</label>
      <InfoTooltip label={label} text={hint} />
    </div>
  );
}

/** Editor de agente dentro de um `Dialog`. */
export function AgentEditorDialog({
  open,
  kind,
  initial,
  onCancel,
  onSave,
}: AgentEditorDialogProps) {
  const [draft, setDraft] = useState<AssistantMode>(
    () => initial ?? blankAgent(),
  );
  const [error, setError] = useState<string | null>(null);

  const patch = <K extends keyof AssistantMode>(
    field: K,
    value: AssistantMode[K],
  ) => setDraft((current) => ({ ...current, [field]: value }));

  const submit = () => {
    const name = draft.name.trim();
    if (!name) {
      setError("Dê um nome ao agente.");
      return;
    }
    if (!hasAnyPrompt(draft)) {
      setError("Preencha ao menos um prompt ou o contexto.");
      return;
    }
    setError(null);
    onSave({ ...draft, name, category: draft.category.trim() });
  };

  const title = kind === "edit" ? "Editar agente" : "Novo assistente de IA";
  const subtitle =
    kind === "edit"
      ? "Altere as propriedades deste agente."
      : "Configure uma persona sob medida para as suas reuniões.";

  return (
    <Dialog
      open={open}
      title={title}
      subtitle={subtitle}
      onClose={onCancel}
      footer={
        <>
          <button
            type="button"
            className="settings__button"
            onClick={onCancel}
          >
            Cancelar
          </button>
          <button
            type="submit"
            form="agent-editor-form"
            className="settings__button settings__button--primary"
          >
            Salvar agente
          </button>
        </>
      }
    >
      <form
        id="agent-editor-form"
        className="agent-editor"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <label className="settings__field">
          <span>Nome</span>
          <input
            type="text"
            autoFocus
            maxLength={MODE_NAME_MAX_CHARS}
            placeholder="Ex.: Vendas B2B"
            value={draft.name}
            onChange={(event) => patch("name", event.target.value)}
          />
        </label>

        <div className="settings__field">
          <span>Ícone</span>
          <div
            className="settings__icons"
            role="radiogroup"
            aria-label="Ícone do agente"
          >
            {AGENT_ICON_IDS.map((icon) => (
              <button
                key={icon}
                type="button"
                role="radio"
                aria-checked={draft.icon === icon}
                aria-label={AGENT_ICON_LABELS[icon] ?? icon}
                title={AGENT_ICON_LABELS[icon] ?? icon}
                className={`settings__icon${
                  draft.icon === icon ? " settings__icon--on" : ""
                }`}
                onClick={() => patch("icon", icon)}
              >
                <AgentIcon icon={icon} size={16} />
              </button>
            ))}
          </div>
        </div>

        <div className="settings__field">
          <span>Cor</span>
          <div
            className="settings__colors"
            role="radiogroup"
            aria-label="Cor do agente"
          >
            {AGENT_COLOR_TOKENS.map((color) => (
              <button
                key={color}
                type="button"
                role="radio"
                aria-checked={draft.color === color}
                aria-label={`Cor ${color}`}
                title={color}
                className={`settings__color settings__color--${color}${
                  draft.color === color ? " settings__color--on" : ""
                }`}
                onClick={() => patch("color", color)}
              >
                <AgentIcon icon={draft.icon} size={16} />
              </button>
            ))}
          </div>
        </div>

        <div className="settings__field">
          <HelpLabel
            htmlFor="agent-suggestion-prompt"
            label="Prompt de Sugestão"
            hint="Contrato da sugestão ao vivo: é a base do prompt de sistema quando a IA sugere a próxima fala. O contrato comum de leitura da transcrição (ME/THEY, erros de ASR) é aplicado automaticamente. Vazio = usa o prompt do agente Padrão."
          />
          <textarea
            id="agent-suggestion-prompt"
            rows={5}
            maxLength={MODE_PROMPT_MAX_CHARS}
            placeholder="O contrato da sugestão ao vivo (o que a janela mostra)."
            value={draft.suggestion_prompt}
            onChange={(event) =>
              patch("suggestion_prompt", event.target.value)
            }
          />
        </div>

        <div className="settings__field">
          <HelpLabel
            htmlFor="agent-summary-prompt"
            label="Prompt de Resumo"
            hint="Contrato de resumo deste agente. Hoje é apenas salvo — o resumo ainda usa o contrato global. Vazio = usa o do agente Padrão."
          />
          <textarea
            id="agent-summary-prompt"
            rows={4}
            maxLength={MODE_PROMPT_MAX_CHARS}
            placeholder="O contrato do resumo deste agente (usado por uma feature futura)."
            value={draft.summary_prompt}
            onChange={(event) => patch("summary_prompt", event.target.value)}
          />
        </div>

        <details className="settings__advanced">
          <summary>Campos avançados</summary>
          <div className="settings__field">
            <HelpLabel
              htmlFor="agent-category"
              label="Categoria"
              hint="Rótulo livre para organizar o agente (ex.: engineering, productivity). Não influencia o comportamento da IA."
            />
            <input
              id="agent-category"
              type="text"
              maxLength={60}
              placeholder="Ex.: engineering"
              value={draft.category}
              onChange={(event) => patch("category", event.target.value)}
            />
          </div>
          <div className="settings__field">
            <HelpLabel
              htmlFor="agent-system-prompt"
              label="Prompt de sistema (critérios de domínio)"
              hint="Critérios de domínio que entram em “Domain criteria” no prompt da sugestão: papel, tom e regras. Não é usado pelo resumo nem pela Busca. Vazio = usa o do agente Padrão."
            />
            <textarea
              id="agent-system-prompt"
              rows={4}
              maxLength={MODE_PROMPT_MAX_CHARS}
              value={draft.system_prompt}
              onChange={(event) => patch("system_prompt", event.target.value)}
            />
          </div>
          <div className="settings__field">
            <HelpLabel
              htmlFor="agent-context"
              label="Contexto fixo do agente"
              hint="Fatos estáveis que entram em “Agent context” nos dois passes da sugestão (rápido e completo): produto, empresa e vocabulário. Vazio = nenhum contexto extra."
            />
            <textarea
              id="agent-context"
              rows={3}
              maxLength={MODE_PROMPT_MAX_CHARS}
              value={draft.context}
              onChange={(event) => patch("context", event.target.value)}
            />
          </div>
        </details>

        {error ? (
          <p className="settings__status settings__status--error">{error}</p>
        ) : null}
      </form>
    </Dialog>
  );
}
