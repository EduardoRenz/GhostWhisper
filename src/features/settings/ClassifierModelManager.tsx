import { useState } from "react";
import type { ClassifierModelManifest } from "../../core/domain/classifierModel";
import { formatModelSize } from "../../core/domain/classifierModel";
import {
  progressLabel,
  progressPercent,
  useClassifierModels,
} from "../../core/application/useClassifierModels";
import { useTranslation } from "../../core/l10n";
import { Pill } from "../../components/Pill";

interface ClassifierModelManagerProps {
  /** `suggestion_classifier_model_id` (ex.: `laya-multilingual`). */
  modelId: string;
  /** Checkpoint selecionado no card (`multilingual`/`english`). */
  checkpoint: string;
}

/**
 * Download/ativação/remoção do artefato ONNX (BR-099).
 *
 * Nenhum download silencioso: o botão abre uma confirmação com o tamanho; só o
 * “Baixar” chama o core. O progresso chega por evento e o estado final vem do
 * model manager (fonte da verdade), nunca de um cálculo da UI.
 */
export function ClassifierModelManager({
  modelId,
  checkpoint,
}: ClassifierModelManagerProps) {
  const { t } = useTranslation();
  const models = useClassifierModels();
  const [confirming, setConfirming] = useState(false);

  const recommended =
    models.catalog?.models.find(
      (model) => model.model_id === modelId && model.checkpoint === checkpoint,
    ) ??
    models.catalog?.models.find((model) => model.checkpoint === checkpoint) ??
    models.recommended;

  const installedVersion = recommended
    ? models.installed.find(
        (model) =>
          model.model_id === recommended.model_id &&
          model.version === recommended.version,
      )
    : undefined;
  const isActive = Boolean(
    installedVersion?.active &&
      models.active?.version === installedVersion.version &&
      models.active?.model_id === installedVersion.model_id,
  );

  const label = recommended
    ? recommended.label || recommended.model_id
    : modelId;
  const size = recommended ? formatModelSize(recommended.size_bytes) : null;
  const progressText = progressLabel(models.progress, t);
  const percent = progressPercent(models.progress);

  const start = (model: ClassifierModelManifest) => {
    setConfirming(false);
    void models.install(model.model_id, model.version);
  };

  return (
    <div className="classifier-model">
      <div className="classifier-model__header">
        <span className="classifier-model__title">
          {t("classifier_model_title")}
        </span>
        <Pill tone={isActive ? "accent" : "neutral"} mono>
          {models.active
            ? t("classifier_model_status_active", {
                version: models.active.version,
              })
            : models.backend?.model_bundled
              ? t("classifier_model_status_bundled")
              : t("classifier_model_status_none")}
        </Pill>
        <button
          type="button"
          className="classifier-model__link"
          disabled={models.loading || models.busy}
          onClick={() => void models.refresh()}
        >
          {t("classifier_model_refresh")}
        </button>
      </div>

      {models.error && (
        <div className="classifier-model__error">
          <span>
            {t("classifier_model_catalog_error", { error: models.error })}
          </span>
          <button
            type="button"
            className="classifier-model__link"
            onClick={() => void models.refresh()}
          >
            {t("classifier_model_retry")}
          </button>
        </div>
      )}

      {models.loading && !models.catalog ? (
        <span className="classifier-model__hint">
          {t("classifier_model_loading")}
        </span>
      ) : recommended ? (
        <div className="classifier-model__card">
          <div className="classifier-model__card-main">
            <strong>{label}</strong>
            <span className="classifier-model__meta">
              {recommended.version}
              {size ? ` · ${size}` : ""}
              {recommended.license
                ? ` · ${t("classifier_model_license", {
                    license: recommended.license,
                  })}`
                : ""}
            </span>
          </div>

          {isActive ? (
            <>
              <Pill tone="accent">{t("classifier_model_active_badge")}</Pill>
              <span className="classifier-model__hint">
                {t("classifier_model_restart_hint")}
              </span>
            </>
          ) : installedVersion ? (
            <button
              type="button"
              className="settings__button settings__button--small"
              disabled={models.busy}
              onClick={() =>
                void models.activate(
                  installedVersion.model_id,
                  installedVersion.version,
                )
              }
            >
              {t("classifier_model_activate")}
            </button>
          ) : confirming ? (
            <div className="classifier-model__confirm">
              <span>
                {t("classifier_model_confirm_body", {
                  size: size ?? "—",
                })}
              </span>
              <div className="classifier-model__actions">
                <button
                  type="button"
                  className="settings__button settings__button--small"
                  disabled={models.busy}
                  onClick={() => start(recommended)}
                >
                  {t("classifier_model_confirm")}
                </button>
                <button
                  type="button"
                  className="classifier-model__link"
                  onClick={() => setConfirming(false)}
                >
                  {t("classifier_model_cancel")}
                </button>
              </div>
            </div>
          ) : (
            <button
              type="button"
              className="settings__button settings__button--small"
              disabled={models.busy}
              onClick={() => setConfirming(true)}
            >
              {t("classifier_model_download")}
            </button>
          )}
        </div>
      ) : models.backend &&
        (!models.backend.laya_feature || models.backend.model_bundled) ? null : (
        <span className="classifier-model__hint">
          {t("classifier_model_no_catalog")}
        </span>
      )}

      {progressText && (
        <div className="classifier-model__progress" aria-live="polite">
          <span>{progressText}</span>
          {percent !== null && (
            <div className="classifier-model__bar" role="progressbar">
              <span style={{ width: `${percent}%` }} />
            </div>
          )}
        </div>
      )}

      {models.installed.length > 0 && (
        <div className="classifier-model__versions">
          <span className="classifier-model__hint">
            {t("classifier_model_installed_title")}
          </span>
          <ul>
            {models.installed.map((model) => (
              <li key={`${model.model_id}-${model.version}`}>
                <span className="classifier-model__version">
                  {model.model_id} · {model.version}
                </span>
                {model.active ? (
                  <Pill tone="accent">{t("classifier_model_active_badge")}</Pill>
                ) : (
                  <>
                    <button
                      type="button"
                      className="classifier-model__link"
                      disabled={models.busy}
                      onClick={() =>
                        void models.activate(model.model_id, model.version)
                      }
                    >
                      {t("classifier_model_activate")}
                    </button>
                    <button
                      type="button"
                      className="classifier-model__link"
                      disabled={models.busy}
                      onClick={() =>
                        void models.remove(model.model_id, model.version)
                      }
                    >
                      {t("classifier_model_remove")}
                    </button>
                  </>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      {models.backend && (
        <p
          className={`classifier-model__hint${
            models.backend.laya_feature
              ? ""
              : " classifier-model__hint--warn"
          }`}
        >
          {!models.backend.laya_feature
            ? t("classifier_engine_missing")
            : models.backend.model_active
              ? t("classifier_engine_active")
              : models.backend.model_bundled
                ? t("classifier_engine_bundled")
                : t("classifier_engine_ready")}
        </p>
      )}

      {!models.catalog &&
        !(
          models.backend &&
          (!models.backend.laya_feature || models.backend.model_bundled)
        ) && (
          <p className="classifier-model__hint">
            {t("classifier_model_unavailable_hint")}
          </p>
        )}
    </div>
  );
}
