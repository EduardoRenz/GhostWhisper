import { useState } from "react";
import { BoltIcon, CheckIcon, GripIcon } from "../../components/icons";
import { Pill } from "../../components/Pill";
import {
  type AiSource,
  type AiSourceHealth,
} from "../../core/domain/settings";
import { useTranslation, type TranslationKey, type TranslationParams } from "../../core/l10n";
import { providerShortLabelKey } from "../../core/l10n/labels";

/** Rótulo do estado de saúde (traduzido para a UI). */
function healthLabel(
  health: AiSourceHealth | undefined,
  t: (key: TranslationKey, params?: TranslationParams) => string,
): string {
  if (!health) return t("sources_never_used");
  switch (health.state) {
    case "ready":
      return health.latency_ms !== null
        ? t("sources_connected_latency_latency_ms", {
            latency: health.latency_ms,
          })
        : t("sources_connected");
    case "degraded":
      return t("sources_unstable");
    case "failed":
      return t("sources_failed");
    default:
      return t("sources_never_used");
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
  const { t } = useTranslation();
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
        <span
          className="settings__source-grip"
          aria-hidden="true"
          title={t("sources_drag_reorder")}
        >
          <GripIcon size={16} />
        </span>
        <span className="settings__source-icon" aria-hidden="true">
          <BoltIcon size={16} />
        </span>
        <div className="settings__source-title">
          <strong>
            {source.label || t(providerShortLabelKey(source.kind))}
          </strong>
          <span className="settings__source-sub">
            {t(providerShortLabelKey(source.kind))} ·{" "}
            {source.model || t("sources_no_model")}
          </span>
        </div>
        <div className="settings__source-badges">
          {isPrimary && <Pill tone="accent">{t("sources_primary")}</Pill>}
          {health?.is_fallback && (
            <Pill tone="accent">{t("sources_fallback_active")}</Pill>
          )}
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
            {healthLabel(health, t)}
          </Pill>
        </div>
      </header>

      {editing ? (
        <div className="settings__source-edit">
          <label className="settings__field">
            <span>{t("sources_model")}</span>
            <input
              type="text"
              value={model}
              placeholder={t("model_placeholder")}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => setModel(event.target.value)}
            />
          </label>
          {source.kind === "custom" && (
            <label className="settings__field">
              <span>{t("sources_base_url_openai_format")}</span>
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
              {t("sources_save")}
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
              {t("sources_cancel")}
            </button>
          </div>
        </div>
      ) : (
        <>
          <div className="settings__source-key">
            <span className="settings__source-key-label">
              {hasKey
                ? keyPreview || "••••••••"
                : t("sources_no_key_configured")}
            </span>
            {hasKey && (
              <span className="settings__source-key-store">
                {t("sources_system_keyring")}
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
                {t("sources_make_primary")}
              </button>
            )}
            <button
              type="button"
              className="settings__button"
              disabled={busy || !hasKey}
              onClick={onValidate}
            >
              {t("sources_validate_key")}
            </button>
            <button
              type="button"
              className="settings__button"
              disabled={busy}
              onClick={() => setKeyEditing((value) => !value)}
            >
              {hasKey ? t("sources_change_key") : t("sources_add_key")}
            </button>
            {hasKey && (
              <button
                type="button"
                className="settings__button settings__button--danger"
                disabled={busy}
                onClick={onClearKey}
              >
                {t("sources_remove_key")}
              </button>
            )}
            <button
              type="button"
              className="settings__button"
              disabled={busy}
              onClick={() => setEditing((value) => !value)}
            >
              {t("sources_edit")}
            </button>
            {canRemove && (
              <button
                type="button"
                className="settings__button settings__button--danger"
                disabled={busy}
                onClick={onRemove}
              >
                {t("sources_delete_source")}
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
                <CheckIcon size={14} /> {t("sources_save_key")}
              </button>
            </div>
          )}
        </>
      )}
    </article>
  );
}
