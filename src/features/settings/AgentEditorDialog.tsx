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
import { AgentIcon } from "../../components/AgentIcon";
import { Dialog } from "../../components/Dialog";
import { InfoTooltip } from "../../components/InfoTooltip";
import { useTranslation, type TranslationKey } from "../../core/l10n";
import { agentIconKey } from "../../core/l10n/labels";

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
  labelKey: TranslationKey;
  /** Explicação do campo (tooltip — BR-081). */
  hint: string;
}

/** Rótulo do campo com o ícone de ajuda. */
function HelpLabel({ htmlFor, labelKey, hint }: HelpLabelProps) {
  const { t } = useTranslation();
  const label = t(labelKey);
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
  const { t } = useTranslation();
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
      setError(t("agent_editor_give_agent_name"));
      return;
    }
    if (!hasAnyPrompt(draft)) {
      setError(t("agent_editor_fill_least_one_prompt_context"));
      return;
    }
    setError(null);
    onSave({ ...draft, name, category: draft.category.trim() });
  };

  const title = kind === "edit" ? t("agent_editor_edit_agent") : t("agent_editor_new_ai_assistant");
  const subtitle =
    kind === "edit"
      ? t("agent_editor_change_properties_agent")
      : t("agent_editor_configure_custom_persona_meetings");

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
            {t("agent_editor_cancel")}
          </button>
          <button
            type="submit"
            form="agent-editor-form"
            className="settings__button settings__button--primary"
          >
            {t("agent_editor_save_agent")}
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
          <span>{t("agent_editor_name")}</span>
          <input
            type="text"
            autoFocus
            maxLength={MODE_NAME_MAX_CHARS}
            placeholder={t("agent_editor_e_g_b2b_sales")}
            value={draft.name}
            onChange={(event) => patch("name", event.target.value)}
          />
        </label>

        <div className="settings__field">
          <span>{t("agent_editor_icon")}</span>
          <div
            className="settings__icons"
            role="radiogroup"
            aria-label={t("agent_editor_agent_icon")}
          >
            {AGENT_ICON_IDS.map((icon) => (
              <button
                key={icon}
                type="button"
                role="radio"
                aria-checked={draft.icon === icon}
                aria-label={t(agentIconKey(icon))}
                title={t(agentIconKey(icon))}
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
          <span>{t("agent_editor_color")}</span>
          <div
            className="settings__colors"
            role="radiogroup"
            aria-label={t("agent_editor_agent_color")}
          >
            {AGENT_COLOR_TOKENS.map((color) => (
              <button
                key={color}
                type="button"
                role="radio"
                aria-checked={draft.color === color}
                aria-label={t("agent_editor_color_color", { color })}
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
            labelKey="agent_editor_suggestion_prompt"
            hint={t(
              "agent_editor_live_suggestion_contract_basis_system_prompt",
            )}
          />
          <textarea
            id="agent-suggestion-prompt"
            rows={5}
            maxLength={MODE_PROMPT_MAX_CHARS}
            placeholder={t(
              "agent_editor_live_suggestion_contract_what_window_shows",
            )}
            value={draft.suggestion_prompt}
            onChange={(event) =>
              patch("suggestion_prompt", event.target.value)
            }
          />
        </div>

        <div className="settings__field">
          <HelpLabel
            htmlFor="agent-summary-prompt"
            labelKey="agent_editor_summary_prompt"
            hint={t("agent_editor_agent_s_summary_contract_used_agent")}
          />
          <textarea
            id="agent-summary-prompt"
            rows={4}
            maxLength={MODE_PROMPT_MAX_CHARS}
            placeholder={t(
              "agent_editor_agent_s_summary_contract_used_future",
            )}
            value={draft.summary_prompt}
            onChange={(event) => patch("summary_prompt", event.target.value)}
          />
        </div>

        <details className="settings__advanced">
          <summary>{t("agent_editor_advanced_fields")}</summary>
          <div className="settings__field">
            <HelpLabel
              htmlFor="agent-category"
              labelKey="agent_editor_category"
              hint={t("agent_editor_free_form_label_organize_agent_e")}
            />
            <input
              id="agent-category"
              type="text"
              maxLength={60}
              placeholder={t("agent_editor_e_g_engineering")}
              value={draft.category}
              onChange={(event) => patch("category", event.target.value)}
            />
          </div>
          <div className="settings__field">
            <HelpLabel
              htmlFor="agent-system-prompt"
              labelKey="agent_editor_system_prompt"
              hint={t("agent_editor_domain_criteria_go_into_domain_criteria")}
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
              labelKey="agent_editor_context"
              hint={t("agent_editor_stable_facts_go_into_agent_context")}
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
