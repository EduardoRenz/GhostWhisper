/**
 * Aba "Assistentes" (BR-074).
 *
 * Painel de gestão dos agentes, na forma do mockup:
 *
 * 1. o agente **Padrão** no topo, protegido (sem editar/excluir);
 * 2. "Agentes configurados (n)" — um card por agente do usuário;
 * 3. "Criar Novo Assistente de IA" (o mesmo modal da edição).
 *
 * A badge "Ativo" é **informativa**: a troca do agente acontece no seletor
 * global do notch (BR-080).
 */

import { useState } from "react";
import { useAssistantModes } from "../../core/application/useAssistantModes";
import {
  PROTECTED_MODE_ID,
  isProtectedMode,
  type AssistantMode,
} from "../../core/domain/assistant";
import type { Settings } from "../../core/domain/settings";
import { useTranslation } from "../../core/l10n";
import { Pill } from "../../components/Pill";
import { PlusIcon, StarIcon } from "../../components/icons";
import { AgentCard } from "./AgentCard";
import { AgentEditorDialog, blankAgent, newAgentId } from "./AgentEditorDialog";

interface AssistantsSettingsProps {
  settings: Settings;
  update: <K extends keyof Settings>(field: K, value: Settings[K]) => void;
}

interface EditorState {
  kind: "create" | "edit";
  mode: AssistantMode;
}

/** Painel de agentes das Configurações. */
export function AssistantsSettings({
  settings,
  update,
}: AssistantsSettingsProps) {
  const { t } = useTranslation();
  const { modes: catalog } = useAssistantModes();
  const [editor, setEditor] = useState<EditorState | null>(null);

  const agents = settings.custom_assistant_modes;
  const protectedMode = catalog.find((mode) => isProtectedMode(mode.id));
  const protectedName =
    protectedMode?.name ?? t("assistants_default_agent");
  const protectedActive = isProtectedMode(settings.assistant_mode_id);

  const startNew = () => setEditor({ kind: "create", mode: blankAgent() });

  const edit = (mode: AssistantMode) =>
    setEditor({ kind: "edit", mode: { ...mode } });

  const duplicate = (mode: AssistantMode) =>
    setEditor({
      kind: "create",
      mode: {
        ...mode,
        id: newAgentId(),
        name: `${mode.name} ${t("assistants_copy")}`,
      },
    });

  const save = (mode: AssistantMode) => {
    const exists = agents.some((agent) => agent.id === mode.id);
    update(
      "custom_assistant_modes",
      exists
        ? agents.map((agent) => (agent.id === mode.id ? mode : agent))
        : [...agents, mode],
    );
    setEditor(null);
  };

  const remove = (id: string) => {
    update(
      "custom_assistant_modes",
      agents.filter((agent) => agent.id !== id),
    );
    // O agente ativo saiu: volta para o Padrão (BR-074).
    if (settings.assistant_mode_id === id) {
      update("assistant_mode_id", PROTECTED_MODE_ID);
    }
  };

  return (
    <section className="settings__agents">
      <article className="agent-default" aria-label={t("assistants_default_assistant")}>
        <header className="agent-default__head">
          <span className="agent-default__icon" aria-hidden="true">
            <StarIcon size={18} />
          </span>
          <h3 className="agent-default__name">{protectedName}</h3>
          <Pill mono>{t("assistants_primary")}</Pill>
          {protectedActive ? (
            <Pill tone="accent" mono>
              {t("assistants_active")}
            </Pill>
          ) : null}
        </header>
        <p className="agent-default__description">
          {t("assistants_system_default_automatically_adapts_tries_understand",
          )}
        </p>
      </article>

      <h2 className="settings__agents-title">
        {t("assistants_configured_agents_count", { count: agents.length })}
      </h2>

      {agents.length === 0 ? (
        <p className="settings__hint">
          {t("assistants_no_agents_configured_yet_create_assistant")}
        </p>
      ) : (
        <ul className="settings__agents-list">
          {agents.map((mode) => (
            <li key={mode.id}>
              <AgentCard
                mode={mode}
                defaultMode={protectedMode}
                active={mode.id === settings.assistant_mode_id}
                onEdit={() => edit(mode)}
                onDuplicate={() => duplicate(mode)}
                onDelete={() => remove(mode.id)}
              />
            </li>
          ))}
        </ul>
      )}

      <div className="agent-cta">
        <button
          type="button"
          className="agent-cta__button"
          onClick={startNew}
        >
          <span className="agent-cta__plus" aria-hidden="true">
            <PlusIcon size={16} />
          </span>
          <span>{t("assistants_create_new_ai_assistant")}</span>
        </button>
        <span className="agent-cta__note">
          {t("assistants_configure_custom_personas_meetings")}
        </span>
      </div>

      <AgentEditorDialog
        key={editor ? `${editor.kind}-${editor.mode.id}` : "closed"}
        open={editor !== null}
        kind={editor?.kind ?? "create"}
        initial={editor?.mode ?? null}
        onCancel={() => setEditor(null)}
        onSave={save}
      />
    </section>
  );
}
