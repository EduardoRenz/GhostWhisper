import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { isTauriRuntime } from "../../adapters/ipc/client";
import { recordingCapabilities } from "../../adapters/ipc/recording";
import { estimateSizePerHour } from "../../core/domain/recording";
import type { RecordingCapabilities } from "../../core/domain/recording";
import {
  RECORDING_AUDIO_KBPS_STEPS,
  RECORDING_CONTAINER_LABELS,
  RECORDING_CONTAINERS,
  RECORDING_ENCODER_LABELS,
  RECORDING_ENCODERS,
  RECORDING_FPS_STEPS,
  RECORDING_MODE_LABELS,
  RECORDING_MODES,
  RECORDING_QUALITIES,
  RECORDING_QUALITY_LABELS,
  RECORDING_RESOLUTION_LABELS,
  RECORDING_RESOLUTIONS,
  RECORDING_TARGET_LABELS,
  RECORDING_TARGETS,
  type Settings,
} from "../../core/domain/settings";

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
  const encoderLabel = capabilities
    ? RECORDING_ENCODER_LABELS[
        capabilities.encoder === "vaapi" ? "vaapi" : "x264"
      ]
    : "detectando…";
  // Pediu VAAPI, mas o build/dispositivo não tem: cai para x264.
  const encoderFallback =
    settings.recording_encoder === "vaapi" && capabilities?.encoder === "x264";

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
        <h2 className="settings__section-title">Gravação da sessão</h2>
        <span className="settings__section-note">
          Aplica-se à próxima sessão
        </span>
      </header>

      <div className="settings__grid">
        <article className="settings__card">
          <header className="settings__card-header">
            <h3 className="settings__card-title">O que gravar</h3>
          </header>
          <p className="settings__card-description">
            A gravação acompanha a sessão: pausa e retoma junto com ela.
          </p>

          <div
            className="settings__segmented"
            role="group"
            aria-label="O que gravar"
          >
            {RECORDING_MODES.map((mode) => (
              <button
                key={mode}
                type="button"
                className={segmentClass(settings.recording_mode === mode)}
                aria-pressed={settings.recording_mode === mode}
                onClick={() => update("recording_mode", mode)}
              >
                {RECORDING_MODE_LABELS[mode]}
              </button>
            ))}
          </div>

          <div className="settings__field settings__field--row">
            <span>Gravar ao iniciar a sessão</span>
            <label className="settings__toggle">
              <input
                type="checkbox"
                aria-label="Gravar ao iniciar a sessão"
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
            <h3 className="settings__card-title">Alvo da captura</h3>
            <span className="settings__badge">
              {RECORDING_TARGET_LABELS[settings.recording_target]}
            </span>
          </header>
          <p className="settings__card-description">
            A janela da reunião não inclui o overlay do GhostWhisper. A tela
            inteira inclui.
          </p>
          <div
            className="settings__segmented"
            role="group"
            aria-label="Alvo da captura"
          >
            {RECORDING_TARGETS.map((target) => (
              <button
                key={target}
                type="button"
                className={segmentClass(settings.recording_target === target)}
                aria-pressed={settings.recording_target === target}
                onClick={() => update("recording_target", target)}
              >
                {RECORDING_TARGET_LABELS[target]}
              </button>
            ))}
          </div>
        </article>

        {isVideo ? (
          <article className="settings__card">
            <header className="settings__card-header">
              <h3 className="settings__card-title">Vídeo</h3>
              <span className="settings__badge">
                {RECORDING_RESOLUTION_LABELS[settings.recording_resolution]}
              </span>
            </header>
            <p className="settings__card-description">
              Resolução nativa preserva a leitura de texto; o FPS define o custo.
            </p>

            <div className="settings__recording-field">
              <span className="settings__recording-label">Resolução</span>
              <div className="settings__segmented" role="group" aria-label="Resolução">
                {RECORDING_RESOLUTIONS.map((resolution) => (
                  <button
                    key={resolution}
                    type="button"
                    className={segmentClass(settings.recording_resolution === resolution)}
                    aria-pressed={settings.recording_resolution === resolution}
                    onClick={() => update("recording_resolution", resolution)}
                  >
                    {RECORDING_RESOLUTION_LABELS[resolution]}
                  </button>
                ))}
              </div>
            </div>

            <div className="settings__recording-field">
              <span className="settings__recording-label">
                FPS <span className="settings__badge">{settings.recording_fps}</span>
              </span>
              <div className="settings__segmented" role="group" aria-label="FPS">
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
              <span className="settings__recording-label">Qualidade</span>
              <div className="settings__segmented" role="group" aria-label="Qualidade">
                {RECORDING_QUALITIES.map((quality) => (
                  <button
                    key={quality}
                    type="button"
                    className={segmentClass(settings.recording_quality === quality)}
                    aria-pressed={settings.recording_quality === quality}
                    onClick={() => update("recording_quality", quality)}
                  >
                    {RECORDING_QUALITY_LABELS[quality]}
                  </button>
                ))}
              </div>
            </div>

            <div className="settings__recording-field">
              <span className="settings__recording-label">Encoder</span>
              <div className="settings__segmented" role="group" aria-label="Encoder">
                {RECORDING_ENCODERS.map((encoder) => (
                  <button
                    key={encoder}
                    type="button"
                    className={segmentClass(settings.recording_encoder === encoder)}
                    aria-pressed={settings.recording_encoder === encoder}
                    onClick={() => update("recording_encoder", encoder)}
                  >
                    {RECORDING_ENCODER_LABELS[encoder]}
                  </button>
                ))}
              </div>
            </div>
          </article>
        ) : null}

        <article className="settings__card">
          <header className="settings__card-header">
            <h3 className="settings__card-title">Áudio</h3>
            <span className="settings__badge">
              {isVideo ? `${settings.recording_audio_kbps} kbps` : "FLAC"}
            </span>
          </header>
          <p className="settings__card-description">
            Microfone e áudio do sistema mixados; sem o ganho/gate do ASR.
          </p>

          {isVideo ? (
            <div className="settings__segmented" role="group" aria-label="Bitrate do áudio">
              {RECORDING_AUDIO_KBPS_STEPS.map((kbps) => (
                <button
                  key={kbps}
                  type="button"
                  className={segmentClass(settings.recording_audio_kbps === kbps)}
                  aria-pressed={settings.recording_audio_kbps === kbps}
                  onClick={() => update("recording_audio_kbps", kbps)}
                >
                  {kbps}k
                </button>
              ))}
            </div>
          ) : (
            <p className="settings__card-description">
              FLAC sem perda, mono 48 kHz (≈ 117 MB por hora).
            </p>
          )}
        </article>

        <article className="settings__card">
          <header className="settings__card-header">
            <h3 className="settings__card-title">Arquivo</h3>
            <span className="settings__badge">
              {RECORDING_CONTAINER_LABELS[settings.recording_container]}
            </span>
          </header>

          <div className="settings__segmented" role="group" aria-label="Container">
            {RECORDING_CONTAINERS.map((container) => (
              <button
                key={container}
                type="button"
                className={segmentClass(settings.recording_container === container)}
                aria-pressed={settings.recording_container === container}
                onClick={() => update("recording_container", container)}
              >
                {RECORDING_CONTAINER_LABELS[container]}
              </button>
            ))}
          </div>

          <label className="settings__field">
            Pasta de destino
            <input
              type="text"
              value={settings.recording_dir ?? ""}
              placeholder="Padrão: ~/Vídeos/GhostWhisper"
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
            Escolher pasta…
          </button>

          <div className="settings__field settings__field--row">
            <span>Manter os segmentos</span>
            <label className="settings__toggle">
              <input
                type="checkbox"
                aria-label="Manter os segmentos"
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
            <h3 className="settings__card-title">Custo estimado</h3>
            <span className="settings__badge">
              {estimate.min}–{estimate.max} MB/h
            </span>
          </header>
          <p className="settings__card-description">
            Encoder detectado: <strong>{encoderLabel}</strong>.
          </p>
          {encoderFallback ? (
            <p className="settings__card-description settings__card-description--warn">
              VAAPI indisponível neste ambiente; a gravação usará x264.
            </p>
          ) : null}
          {capabilities && !capabilities.available ? (
            <p className="settings__card-description settings__card-description--warn">
              {capabilities.reason ?? "Gravação indisponível neste ambiente."}
            </p>
          ) : null}
        </article>
      </div>
    </section>
  );
}
