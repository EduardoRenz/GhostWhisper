import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { isTauriRuntime } from "../../adapters/ipc/client";
import { recordingCapabilities } from "../../adapters/ipc/recording";
import { estimateSizePerHour } from "../../core/domain/recording";
import type { RecordingCapabilities } from "../../core/domain/recording";
import {
  RECORDING_AUDIO_KBPS_STEPS,
  RECORDING_CONTAINERS,
  RECORDING_ENCODERS,
  RECORDING_FPS_STEPS,
  RECORDING_MODES,
  RECORDING_QUALITIES,
  RECORDING_RESOLUTIONS,
  RECORDING_TARGETS,
  type Settings,
} from "../../core/domain/settings";
import { useTranslation } from "../../core/l10n";
import {
  recordingContainerKey,
  recordingEncoderKey,
  recordingModeKey,
  recordingQualityKey,
  recordingResolutionKey,
  recordingTargetKey,
  recordingUnavailableKey,
} from "../../core/l10n/labels";

interface RecordingSettingsProps {
  settings: Settings;
  update: <K extends keyof Settings>(field: K, value: Settings[K]) => void;
}

/**
 * Aba "Gravação" (BR-066).
 *
 * Controla o custo e a qualidade do vídeo, o áudio, o arquivo e o alvo. Os
 * padrões vêm da máquina de referência: 1080p nativo · 8 fps · CRF 27.
 */
export function RecordingSettings({ settings, update }: RecordingSettingsProps) {
  const { t } = useTranslation();
  const [capabilities, setCapabilities] =
    useState<RecordingCapabilities | null>(null);

  useEffect(() => {
    let cancelled = false;
    void recordingCapabilities().then((value) => {
      if (!cancelled && value) {
        setCapabilities(value);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const segmentClass = (active: boolean) =>
    active
      ? "settings__segment settings__segment--active"
      : "settings__segment";

  const estimate = estimateSizePerHour(settings);
  const isVideo = settings.recording_mode === "video";
  // Wayland: a fonte é escolhida no diálogo do portal a cada gravação.
  const portalTargets = capabilities?.target_mode === "portal";
  const encoderLabel = capabilities
    ? t(
        recordingEncoderKey(
          capabilities.encoder === "vaapi" ? "vaapi" : "x264",
        ),
      )
    : t("recording_settings_detecting");
  // Pediu VAAPI, mas o build/dispositivo não tem: cai para x264.
  const encoderFallback =
    settings.recording_encoder === "vaapi" && capabilities?.encoder === "x264";
  // Motivo da indisponibilidade em l10n (código estável) com fallback ao texto.
  const unavailableKey = recordingUnavailableKey(capabilities?.reason_code);
  const unavailableText = unavailableKey
    ? t(unavailableKey)
    : (capabilities?.reason ??
      t("recording_settings_recording_unavailable_environment"));

  const pickFolder = async () => {
    if (!isTauriRuntime()) {
      return;
    }
    const chosen = await open({ directory: true, multiple: false });
    if (typeof chosen === "string") {
      update("recording_dir", chosen);
    }
  };

  return (
    <section className="settings__recording">
      <header className="settings__section-header">
        <h2 className="settings__section-title">
          {t("recording_settings_session_recording")}
        </h2>
        <span className="settings__section-note">
          {t("recording_settings_applies_next_session")}
        </span>
      </header>

      <div className="settings__grid">
        <article className="settings__card">
          <header className="settings__card-header">
            <h3 className="settings__card-title">{t("recording_settings_what_record")}</h3>
          </header>
          <p className="settings__card-description">
            {t("recording_settings_recording_follows_session_pauses_resumes",
            )}
          </p>

          <div
            className="settings__segmented"
            role="group"
            aria-label={t("recording_settings_what_record")}
          >
            {RECORDING_MODES.map((mode) => (
              <button
                key={mode}
                type="button"
                className={segmentClass(settings.recording_mode === mode)}
                aria-pressed={settings.recording_mode === mode}
                onClick={() => update("recording_mode", mode)}
              >
                {t(recordingModeKey(mode))}
              </button>
            ))}
          </div>

          <div className="settings__field settings__field--row">
            <span>{t("recording_settings_record_session_starts")}</span>
            <label className="settings__toggle">
              <input
                type="checkbox"
                aria-label={t("recording_settings_record_session_starts")}
                checked={settings.recording_auto_start}
                onChange={(event) =>
                  update("recording_auto_start", event.target.checked)
                }
              />
              <span className="settings__toggle-track" aria-hidden="true" />
            </label>
          </div>
        </article>

        <article className="settings__card">
          <header className="settings__card-header">
            <h3 className="settings__card-title">{t("recording_settings_capture_target")}</h3>
            <span className="settings__badge">
              {portalTargets
                ? t("recording_target_portal")
                : t(recordingTargetKey(settings.recording_target))}
            </span>
          </header>
          {portalTargets ? (
            <p className="settings__card-description">
              {t("recording_settings_portal_target_description")}
            </p>
          ) : (
            <>
              <p className="settings__card-description">
                {t("recording_settings_meeting_window_not_include_ghostwhisper_overlay",
                )}
              </p>
              <div
                className="settings__segmented"
                role="group"
                aria-label={t("recording_settings_capture_target")}
              >
                {RECORDING_TARGETS.map((target) => (
                  <button
                    key={target}
                    type="button"
                    className={segmentClass(settings.recording_target === target)}
                    aria-pressed={settings.recording_target === target}
                    onClick={() => update("recording_target", target)}
                  >
                    {t(recordingTargetKey(target))}
                  </button>
                ))}
              </div>

              <div className="settings__field settings__field--row">
                <span>{t("recording_settings_follow_active_window")}</span>
                <label className="settings__toggle">
                  <input
                    type="checkbox"
                    aria-label={t("recording_settings_follow_active_window_while_recording")}
                    checked={settings.recording_follow_active}
                    onChange={(event) =>
                      update("recording_follow_active", event.target.checked)
                    }
                  />
                  <span className="settings__toggle-track" aria-hidden="true" />
                </label>
              </div>
              <p className="settings__card-description">
                {t("recording_settings_switches_target_focus_changes_1_s",
                )}
              </p>
            </>
          )}
        </article>

        {isVideo ? (
          <article className="settings__card">
            <header className="settings__card-header">
              <h3 className="settings__card-title">{t("recording_settings_video")}</h3>
              <span className="settings__badge">
                {t(recordingResolutionKey(settings.recording_resolution))}
              </span>
            </header>
            <p className="settings__card-description">
              {t("recording_settings_native_resolution_preserves_text_legibility_fps",
              )}
            </p>

            <div className="settings__recording-field">
              <span className="settings__recording-label">
                {t("recording_settings_resolution")}
              </span>
              <div
                className="settings__segmented"
                role="group"
                aria-label={t("recording_settings_resolution")}
              >
                {RECORDING_RESOLUTIONS.map((resolution) => (
                  <button
                    key={resolution}
                    type="button"
                    className={segmentClass(
                      settings.recording_resolution === resolution,
                    )}
                    aria-pressed={
                      settings.recording_resolution === resolution
                    }
                    onClick={() => update("recording_resolution", resolution)}
                  >
                    {t(recordingResolutionKey(resolution))}
                  </button>
                ))}
              </div>
            </div>

            <div className="settings__recording-field">
              <span className="settings__recording-label">
                {t("recording_fps_label")}{" "}
                <span className="settings__badge">{settings.recording_fps}</span>
              </span>
              <div
                className="settings__segmented"
                role="group"
                aria-label={t("recording_fps_label")}
              >
                {RECORDING_FPS_STEPS.map((fps) => (
                  <button
                    key={fps}
                    type="button"
                    className={segmentClass(settings.recording_fps === fps)}
                    aria-pressed={settings.recording_fps === fps}
                    onClick={() => update("recording_fps", fps)}
                  >
                    {fps}
                  </button>
                ))}
              </div>
            </div>

            <div className="settings__recording-field">
              <span className="settings__recording-label">
                {t("recording_settings_quality")}
              </span>
              <div
                className="settings__segmented"
                role="group"
                aria-label={t("recording_settings_quality")}
              >
                {RECORDING_QUALITIES.map((quality) => (
                  <button
                    key={quality}
                    type="button"
                    className={segmentClass(
                      settings.recording_quality === quality,
                    )}
                    aria-pressed={settings.recording_quality === quality}
                    onClick={() => update("recording_quality", quality)}
                  >
                    {t(recordingQualityKey(quality))}
                  </button>
                ))}
              </div>
            </div>

            <div className="settings__recording-field">
              <span className="settings__recording-label">
                {t("recording_encoder_label")}
              </span>
              <div
                className="settings__segmented"
                role="group"
                aria-label={t("recording_encoder_label")}
              >
                {RECORDING_ENCODERS.map((encoder) => (
                  <button
                    key={encoder}
                    type="button"
                    className={segmentClass(
                      settings.recording_encoder === encoder,
                    )}
                    aria-pressed={settings.recording_encoder === encoder}
                    onClick={() => update("recording_encoder", encoder)}
                  >
                    {t(recordingEncoderKey(encoder))}
                  </button>
                ))}
              </div>
            </div>
          </article>
        ) : null}

        <article className="settings__card">
          <header className="settings__card-header">
            <h3 className="settings__card-title">{t("recording_settings_audio")}</h3>
            <span className="settings__badge">
              {isVideo
                ? `${settings.recording_audio_kbps} kbps`
                : t("media_flac_label")}
            </span>
          </header>
          <p className="settings__card-description">
            {t("recording_settings_microphone_system_audio_mixed_without_asr",
            )}
          </p>

          {isVideo ? (
            <div
              className="settings__segmented"
              role="group"
              aria-label={t("recording_settings_audio_bitrate")}
            >
              {RECORDING_AUDIO_KBPS_STEPS.map((kbps) => (
                <button
                  key={kbps}
                  type="button"
                  className={segmentClass(
                    settings.recording_audio_kbps === kbps,
                  )}
                  aria-pressed={settings.recording_audio_kbps === kbps}
                  onClick={() => update("recording_audio_kbps", kbps)}
                >
                  {kbps}k
                </button>
              ))}
            </div>
          ) : (
            <p className="settings__card-description">
              {t("recording_settings_lossless_flac_mono_48_khz_117")}
            </p>
          )}
        </article>

        <article className="settings__card">
          <header className="settings__card-header">
            <h3 className="settings__card-title">{t("recording_settings_file")}</h3>
            <span className="settings__badge">
              {t(recordingContainerKey(settings.recording_container))}
            </span>
          </header>

          <div
            className="settings__segmented"
            role="group"
            aria-label={t("recording_container_label")}
          >
            {RECORDING_CONTAINERS.map((container) => (
              <button
                key={container}
                type="button"
                className={segmentClass(
                  settings.recording_container === container,
                )}
                aria-pressed={settings.recording_container === container}
                onClick={() => update("recording_container", container)}
              >
                {t(recordingContainerKey(container))}
              </button>
            ))}
          </div>

          <label className="settings__field">
            {t("recording_settings_destination_folder")}
            <input
              type="text"
              value={settings.recording_dir ?? ""}
              placeholder={t("recording_settings_default_videos_ghostwhisper")}
              onChange={(event) =>
                update(
                  "recording_dir",
                  event.target.value.trim() === "" ? null : event.target.value,
                )
              }
            />
          </label>
          <button
            type="button"
            className="settings__segment"
            onClick={() => void pickFolder()}
          >
            {t("recording_settings_choose_folder")}
          </button>

          <div className="settings__field settings__field--row">
            <span>{t("recording_settings_keep_segments")}</span>
            <label className="settings__toggle">
              <input
                type="checkbox"
                aria-label={t("recording_settings_keep_segments")}
                checked={settings.recording_keep_segments}
                onChange={(event) =>
                  update("recording_keep_segments", event.target.checked)
                }
              />
              <span className="settings__toggle-track" aria-hidden="true" />
            </label>
          </div>
        </article>

        <article className="settings__card settings__card--hud">
          <header className="settings__card-header">
            <h3 className="settings__card-title">{t("recording_settings_estimated_cost")}</h3>
            <span className="settings__badge">
              {estimate.min}–{estimate.max} MB/h
            </span>
          </header>
          <p className="settings__card-description">
            {t("recording_settings_detected_encoder_encoder", { encoder: encoderLabel })}
          </p>
          {encoderFallback ? (
            <p className="settings__card-description settings__card-description--warn">
              {t("recording_settings_vaapi_unavailable_environment_recording_use_x264")}
            </p>
          ) : null}
          {capabilities && !capabilities.available ? (
            <p className="settings__card-description settings__card-description--warn">
              {unavailableText}
            </p>
          ) : null}
        </article>
      </div>
    </section>
  );
}
