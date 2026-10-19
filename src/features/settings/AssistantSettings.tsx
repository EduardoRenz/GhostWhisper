import { useEffect, useState } from "react";
import { useAssistantModes } from "../../core/application/useAssistantModes";
import {
  SUGGESTION_INTERVALS,
  SUMMARY_INTERVALS,
  type Settings,
} from "../../core/domain/settings";
import {
  AGENT_COLOR_TOKENS,
  DEFAULT_AGENT_COLOR,
  DEFAULT_AGENT_ICON,
  MODE_NAME_MAX_CHARS,
  MODE_PROMPT_MAX_CHARS,
  agentColorToken,
  agentIconId,
  isBuiltinMode,
  type AssistantMode,
} from "../../core/domain/assistant";
import { AgentIcon, AGENT_ICON_LABELS, AGENT_ICON_OPTIONS } from "../../components/AgentIcon";

interface AssistantSettingsProps {
  settings: Settings;
  update: <K extends keyof Settings>(field: K, value: Settings[K]) => void;
}

/** Gera um id estável para um agente personalizado. */
function newModeId(): string {
  const random =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.round(Math.random() * 1e6)}`;
  return `custom-${random}`;
}

/** Aba “Assistente”: agente padrão, cadências e agentes personalizados. */
export function AssistantSettings({ settings, update }: AssistantSettingsProps) {
  const { modes: catalog } = useAssistantModes(settings.custom_assistant_modes);
  const [draft, setDraft] = useState<AssistantMode | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setError(null);
  }, [draft?.id]);

  const builtins = catalog.filter((mode) => isBuiltinMode(mode.id));
  const custom = settings.custom_assistant_modes;

  const startNew = () => {
    setError(null);
    setDraft({
      id: newModeId(),
      name: "",
      category: "custom",
      system_prompt: "",
      suggestion_prompt: "",
      context: "",
      icon: DEFAULT_AGENT_ICON,
      color: DEFAULT_AGENT_COLOR,
    });
  };

  const duplicate = (mode: AssistantMode) => {
    setError(null);
    setDraft({ ...mode, id: newModeId(), name: `${mode.name} (cópia)` });
  };

  const saveDraft = () => {
    if (!draft) {
      return;
    }
    const name = draft.name.trim();
    if (!name) {
      setError("Dê um nome ao agente.");
      return;
    }
    if (
      !draft.system_prompt.trim() &&
      !draft.suggestion_prompt.trim() &&
      !draft.context.trim()
    ) {
      setError("Preencha ao menos um prompt ou o contexto.");
      return;
    }
    const normalized: AssistantMode = { ...draft, name };
    const exists = custom.some((mode) => mode.id === normalized.id);
    const next = exists
      ? custom.map((mode) => (mode.id === normalized.id ? normalized : mode))
      : [...custom, normalized];
    update("custom_assistant_modes", next);
    setDraft(null);
    setError(null);
  };

  const remove = (id: string) => {
    update(
      "custom_assistant_modes",
      custom.filter((mode) => mode.id !== id),
    );
    if (settings.assistant_mode_id === id) {
      update("assistant_mode_id", "meeting");
    }
  };

  return (
    <>
      <label className="settings__field">
        <span>Agente padrão</span>
        <select
          value={settings.assistant_mode_id}
          onChange={(event) => update("assistant_mode_id", event.target.value)}
        >
          {builtins.map((mode) => (
            <option key={mode.id} value={mode.id}>
              {mode.name}
            </option>
          ))}
          {custom.map((mode) => (
            <option key={mode.id} value={mode.id}>
              {mode.name}
            </option>
          ))}
        </select>
      </label>

      <div className="settings__field settings__field--row">
        <input
          type="checkbox"
          checked={settings.summary_auto_refresh}
          onChange={(event) =>
            update("summary_auto_refresh", event.target.checked)
          }
        />
        <span>Resumo automático</span>
      </div>
      <label className="settings__field">
        <span>Intervalo do resumo</span>
        <select
          value={settings.summary_interval_secs}
          onChange={(event) =>
            update("summary_interval_secs", Number(event.target.value))
          }
        >
          {SUMMARY_INTERVALS.map((seconds) => (
            <option key={seconds} value={seconds}>
              {seconds}s
            </option>
          ))}
        </select>
      </label>

      <div className="settings__field settings__field--row">
        <input
          type="checkbox"
          checked={settings.suggestion_auto_refresh}
          onChange={(event) =>
            update("suggestion_auto_refresh", event.target.checked)
          }
        />
        <span>Sugestões automáticas</span>
      </div>
      <label className="settings__field">
        <span>Intervalo das sugestões</span>
        <select
          value={settings.suggestion_interval_secs}
          onChange={(event) =>
            update("suggestion_interval_secs", Number(event.target.value))
          }
        >
          {SUGGESTION_INTERVALS.map((seconds) => (
            <option key={seconds} value={seconds}>
              {seconds}s
            </option>
          ))}
        </select>
      </label>

      <span className="settings__group-title">Agentes personalizados</span>
      <ul className="settings__modes">
        {custom.length === 0 && (
          <li className="settings__hint">
            Nenhum agente personalizado ainda.
          </li>
        )}
        {custom.map((mode) => (
          <li key={mode.id} className="settings__mode">
            <span
              className={`settings__mode-name agent-color--${agentColorToken(mode)}`}
            >
              <AgentIcon icon={agentIconId(mode)} size={14} />
              {mode.name}
            </span>
            <button
              type="button"
              className="settings__button settings__button--small"
              onClick={() => setDraft({ ...mode })}
            >
              Editar
            </button>
            <button
              type="button"
              className="settings__button settings__button--small"
              onClick={() => duplicate(mode)}
            >
              Duplicar
            </button>
            <button
              type="button"
              className="settings__button settings__button--small"
              onClick={() => remove(mode.id)}
            >
              Excluir
            </button>
          </li>
        ))}
      </ul>

      <div className="settings__actions">
        <button
          type="button"
          className="settings__button"
          onClick={startNew}
        >
          Novo agente
        </button>
        {builtins.map((mode) => (
          <button
            key={mode.id}
            type="button"
            className="settings__button settings__button--small"
            onClick={() => duplicate(mode)}
            title="Cria um agente personalizado a partir deste"
          >
            Duplicar {mode.name}
          </button>
        ))}
      </div>

      {draft && (
        <div className="settings__editor">
          <label className="settings__field">
            <span>Nome</span>
            <input
              type="text"
              maxLength={MODE_NAME_MAX_CHARS}
              value={draft.name}
              onChange={(event) =>
                setDraft({ ...draft, name: event.target.value })
              }
            />
          </label>
          <label className="settings__field">
            <span>Categoria</span>
            <input
              type="text"
              maxLength={60}
              value={draft.category}
              onChange={(event) =>
                setDraft({ ...draft, category: event.target.value })
              }
            />
          </label>
          <div className="settings__field">
            <span>Ícone</span>
            <div
              className="settings__icons"
              role="radiogroup"
              aria-label="Ícone do agente"
            >
              {AGENT_ICON_OPTIONS.map((icon) => (
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
                  onClick={() => setDraft({ ...draft, icon })}
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
                  aria-label={color}
                  title={color}
                  className={`settings__color settings__color--${color}${
                    draft.color === color ? " settings__color--on" : ""
                  }`}
                  onClick={() => setDraft({ ...draft, color })}
                >
                  <AgentIcon icon={draft.icon} size={16} />
                </button>
              ))}
            </div>
          </div>
          <label className="settings__field">
            <span>Prompt de sistema (respostas escritas / critérios)</span>
            <textarea
              rows={4}
              maxLength={MODE_PROMPT_MAX_CHARS}
              value={draft.system_prompt}
              onChange={(event) =>
                setDraft({ ...draft, system_prompt: event.target.value })
              }
            />
          </label>
          <label className="settings__field">
            <span>Prompt de sugestão ao vivo</span>
            <textarea
              rows={5}
              maxLength={MODE_PROMPT_MAX_CHARS}
              value={draft.suggestion_prompt}
              onChange={(event) =>
                setDraft({ ...draft, suggestion_prompt: event.target.value })
              }
            />
          </label>
          <label className="settings__field">
            <span>Contexto fixo do agente</span>
            <textarea
              rows={3}
              maxLength={MODE_PROMPT_MAX_CHARS}
              value={draft.context}
              onChange={(event) =>
                setDraft({ ...draft, context: event.target.value })
              }
            />
          </label>
          {error && <p className="settings__status settings__status--error">{error}</p>}
          <div className="settings__actions">
            <button
              type="button"
              className="settings__button"
              onClick={saveDraft}
            >
              Salvar agente
            </button>
            <button
              type="button"
              className="settings__button"
              onClick={() => setDraft(null)}
            >
              Cancelar
            </button>
          </div>
        </div>
      )}
    </>
  );
}
