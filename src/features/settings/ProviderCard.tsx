import { useState } from "react";
import { BoltIcon, CheckIcon, GripIcon } from "../../components/icons";
import { Pill } from "../../components/Pill";
import {
  MODEL_PLACEHOLDER,
  PROVIDER_SHORT_LABELS,
  type AiSource,
  type AiSourceHealth,
} from "../../core/domain/settings";

/** Rótulo do estado de saúde (traduzido para a UI). */
function healthLabel(health: AiSourceHealth | undefined): string {
  if (!health) return "Nunca usada";
  switch (health.state) {
    case "ready":
      return health.latency_ms !== null
        ? `Conectada · Latência ${health.latency_ms} ms`
        : "Conectada";
    case "degraded":
      return "Instável";
    case "failed":
      return "Falhou";
    default:
      return "Nunca usada";
  }
}

interface ProviderCardProps {
  source: AiSource;
  /** É a fonte primária (a primeira da lista)? */
  isPrimary: boolean;
  /** Informe de saúde (quando conhecido). */
  health?: AiSourceHealth;
  /** A fonte tem chave no keyring? */
  hasKey: boolean;
  /** Versão mascarada da chave. */
  keyPreview: string;
  /** Existe outra fonte (dá para remover esta)? */
  canRemove: boolean;
  /** Busy (uma operação em andamento). */
  busy: boolean;
  /** Torna a fonte primária. */
  onMakePrimary: () => void;
  /** Testa a conexão. */
  onValidate: () => void;
  /** Troca a chave. */
  onChangeKey: (key: string) => void;
  /** Remove a chave. */
  onClearKey: () => void;
  /** Edita modelo/base_url. */
  onEdit: (patch: Partial<AiSource>) => void;
  /** Remove a fonte da lista. */
  onRemove: () => void;
  /** Início do arraste (reordenar). */
  onDragStart: () => void;
  /** Fim do arraste. */
  onDragEnd: () => void;
  /** O card está sob o cursor durante o arraste. */
  onDragOver: () => void;
  /** Solta o arraste sobre este card. */
  onDrop: () => void;
  /** O card é o alvo do arraste? */
  isDropTarget: boolean;
}

/**
 * Card de uma **fonte de IA** (BR-044).
 *
 * Mostra o estado da fonte (primária, conectada, em fallback, falhou), a chave
 * mascarada e as ações: promover, validar, trocar/remover chave, editar e
 * excluir.
 */
export function ProviderCard({
  source,
  isPrimary,
  health,
  hasKey,
  keyPreview,
  canRemove,
  busy,
  onMakePrimary,
  onValidate,
  onChangeKey,
  onClearKey,
  onEdit,
  onRemove,
  onDragStart,
  onDragEnd,
  onDragOver,
  onDrop,
  isDropTarget,
}: ProviderCardProps) {
  const [editing, setEditing] = useState(false);
  const [model, setModel] = useState(source.model);
  const [baseUrl, setBaseUrl] = useState(source.base_url ?? "");
  const [keyInput, setKeyInput] = useState("");
  const [keyEditing, setKeyEditing] = useState(false);

  const submitKey = () => {
    if (keyInput.trim() === "") return;
    onChangeKey(keyInput.trim());
    setKeyInput("");
    setKeyEditing(false);
  };

  const submitEdit = () => {
    onEdit({
      model: model.trim(),
      base_url: baseUrl.trim() === "" ? null : baseUrl.trim(),
    });
    setEditing(false);
  };

  const classes = [
    "settings__source",
    isPrimary ? "settings__source--primary" : "",
    source.enabled ? "" : "settings__source--disabled",
    isDropTarget ? "settings__source--drop" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <article
      className={classes}
      draggable
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onDragOver={(event) => {
        event.preventDefault();
        onDragOver();
      }}
      onDrop={onDrop}
      data-testid={`source-${source.id}`}
    >
      <header className="settings__source-header">
        <span className="settings__source-grip" aria-hidden="true" title="Arraste para reordenar">
          <GripIcon size={16} />
        </span>
        <span className="settings__source-icon" aria-hidden="true">
          <BoltIcon size={16} />
        </span>
        <div className="settings__source-title">
          <strong>{source.label || PROVIDER_SHORT_LABELS[source.kind]}</strong>
          <span className="settings__source-sub">
            {PROVIDER_SHORT_LABELS[source.kind]} · {source.model || "sem modelo"}
          </span>
        </div>
        <div className="settings__source-badges">
          {isPrimary && <Pill tone="accent">Primário</Pill>}
          {health?.is_fallback && <Pill tone="accent">Fallback ativo</Pill>}
          <Pill
            tone={
              health?.state === "ready"
                ? "accent"
                : health?.state === "failed"
                  ? "neutral"
                  : "neutral"
            }
            title={health?.last_error ?? undefined}
            className={`settings__badge--${health?.state ?? "unknown"}`}
          >
            {healthLabel(health)}
          </Pill>
        </div>
      </header>

      {editing ? (
        <div className="settings__source-edit">
          <label className="settings__field">
            <span>Modelo</span>
            <input
              type="text"
              value={model}
              placeholder={MODEL_PLACEHOLDER}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => setModel(event.target.value)}
            />
          </label>
          {source.kind === "custom" && (
            <label className="settings__field">
              <span>Base URL (formato OpenAI)</span>
              <input
                type="url"
                value={baseUrl}
                placeholder="http://localhost:11434/v1"
                onChange={(event) => setBaseUrl(event.target.value)}
              />
            </label>
          )}
          <div className="settings__actions">
            <button
              type="button"
              className="settings__button settings__button--primary"
              onClick={submitEdit}
            >
              Salvar
            </button>
            <button
              type="button"
              className="settings__button"
              onClick={() => {
                setModel(source.model);
                setBaseUrl(source.base_url ?? "");
                setEditing(false);
              }}
            >
              Cancelar
            </button>
          </div>
        </div>
      ) : (
        <>
          <div className="settings__source-key">
            <span className="settings__source-key-label">
              {hasKey ? keyPreview || "••••••••" : "Sem chave configurada"}
            </span>
            {hasKey && (
              <span className="settings__source-key-store">
                Keyring do sistema
              </span>
            )}
          </div>

          <div className="settings__actions">
            {!isPrimary && (
              <button
                type="button"
                className="settings__button"
                disabled={busy}
                onClick={onMakePrimary}
              >
                Tornar primário
              </button>
            )}
            <button
              type="button"
              className="settings__button"
              disabled={busy || !hasKey}
              onClick={onValidate}
            >
              Validar chave
            </button>
            <button
              type="button"
              className="settings__button"
              disabled={busy}
              onClick={() => setKeyEditing((value) => !value)}
            >
              {hasKey ? "Trocar chave" : "Adicionar chave"}
            </button>
            {hasKey && (
              <button
                type="button"
                className="settings__button settings__button--danger"
                disabled={busy}
                onClick={onClearKey}
              >
                Remover chave
              </button>
            )}
            <button
              type="button"
              className="settings__button"
              disabled={busy}
              onClick={() => setEditing((value) => !value)}
            >
              Editar
            </button>
            {canRemove && (
              <button
                type="button"
                className="settings__button settings__button--danger"
                disabled={busy}
                onClick={onRemove}
              >
                Excluir fonte
              </button>
            )}
          </div>

          {keyEditing && (
            <div className="settings__source-key-edit">
              <input
                type="password"
                autoComplete="off"
                placeholder="sk-..."
                value={keyInput}
                onChange={(event) => setKeyInput(event.target.value)}
              />
              <button
                type="button"
                className="settings__button settings__button--primary"
                disabled={busy || keyInput.trim() === ""}
                onClick={submitKey}
              >
                <CheckIcon size={14} /> Salvar chave
              </button>
            </div>
          )}
        </>
      )}
    </article>
  );
}
