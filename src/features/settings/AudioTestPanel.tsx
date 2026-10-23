import { useEffect, useState } from "react";
import {
  listMicrophones,
  listSystemSources,
  monitorDefaultDelay,
  startMicrophone,
  startMonitor,
  startSystemAudio,
  stopMicrophone,
  stopMonitor,
  stopSystemAudio,
  type AudioDevice,
} from "../../adapters/ipc/audio";
import { useAudioLevel } from "../../core/application/useAudioLevel";
import { useSessionState } from "../../core/application/useSessionState";
import { useTranslation } from "../../core/l10n";
import { channelLabelKey } from "../../core/l10n/labels";
import {
  levelPercent,
  type SessionChannel,
} from "../../core/domain/transcript";

/**
 * Teste de captura: escolhe o dispositivo, inicia a gravação e mostra o nível
 * em tempo real — para conferir se o microfone/áudio do sistema está bom antes
 * de abrir uma sessão.
 *
 * No microfone o nível exibido já inclui o **ganho de sensibilidade** e o
 * **noise gate** (BR-018), ou seja, é o que o ASR vai receber.
 */
export function AudioTestPanel({ micGain }: { micGain?: number }) {
  const { t } = useTranslation();
  const [channel, setChannel] = useState<SessionChannel>("microphone");
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [monitoring, setMonitoring] = useState(false);
  const [monitorDelay, setMonitorDelay] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const levels = useAudioLevel();
  const { state } = useSessionState();
  const sessionActive = state.listening;

  // Atraso do monitor (definido pelo core; o usuário não precisa escolher).
  useEffect(() => {
    let cancelled = false;
    void monitorDefaultDelay().then((delay) => {
      if (!cancelled && typeof delay === "number") {
        setMonitorDelay(delay);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  // (Re)carrega dispositivos ao trocar de canal.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const list =
        channel === "microphone"
          ? await listMicrophones()
          : await listSystemSources();
      if (cancelled) return;
      const available = list ?? [];
      setDevices(available);
      setDeviceId(
        available.find((device) => device.is_default)?.id ??
          available[0]?.id ??
          null,
      );
    })();
    return () => {
      cancelled = true;
    };
  }, [channel]);

  // Nunca deixa uma captura de teste (nem o monitor) pendurada ao sair da página.
  useEffect(
    () => () => {
      void stopMonitor();
      void stopMicrophone();
      void stopSystemAudio();
    },
    [],
  );

  // O monitor é só do microfone: ao trocar para o áudio do sistema, desliga.
  useEffect(() => {
    if (channel === "system" && monitoring) {
      void stopMonitor();
      setMonitoring(false);
    }
  }, [channel, monitoring]);

  const onToggleMonitor = async () => {
    setError(null);
    try {
      if (monitoring) {
        await stopMonitor();
        setMonitoring(false);
      } else {
        await startMonitor(monitorDelay ?? undefined);
        setMonitoring(true);
      }
    } catch (caught) {
      setError(String(caught));
      setMonitoring(false);
    }
  };

  const onStart = async () => {
    setError(null);
    try {
      if (channel === "microphone") {
        await startMicrophone(deviceId);
      } else {
        await startSystemAudio(deviceId);
      }
      setRunning(true);
    } catch (caught) {
      setError(String(caught));
      setRunning(false);
    }
  };

  const onStop = async () => {
    setError(null);
    try {
      await stopMonitor();
      setMonitoring(false);
      if (channel === "microphone") {
        await stopMicrophone();
      } else {
        await stopSystemAudio();
      }
    } catch (caught) {
      setError(String(caught));
    } finally {
      setRunning(false);
    }
  };

  const level = levels[channel];
  const percent = running ? levelPercent(level) : 0;
  const silent = running && percent < 2;

  return (
    <section className="settings__group">
      <span className="settings__group-title">{t("audio_test_audio_test")}</span>
      <p className="settings__hint">
        {t("audio_test_check_microphone_system_audio_before_starting",
        )}
      </p>

      <label className="settings__field">
        <span>{t("audio_test_channel")}</span>
        <select
          value={channel}
          disabled={running}
          onChange={(event) =>
            setChannel(event.target.value as SessionChannel)
          }
        >
          <option value="microphone">{t("audio_test_microphone")}</option>
          <option value="system">{t("audio_test_system_audio")}</option>
        </select>
      </label>

      <label className="settings__field">
        <span>{t("audio_test_device")}</span>
        <select
          value={deviceId ?? ""}
          disabled={running || devices.length === 0}
          onChange={(event) => setDeviceId(event.target.value || null)}
        >
          {devices.length === 0 && (
            <option value="">{t("audio_test_none_found")}</option>
          )}
          {devices.map((device) => (
            <option key={device.id} value={device.id}>
              {device.name}
              {device.is_default ? ` ${t("audio_test_default")}` : ""}
            </option>
          ))}
        </select>
      </label>

      <div className="audio-test">
        <div className="levels">
          <span className="levels__label">{t(channelLabelKey(channel))}</span>
          <div className="levels__bar">
            <div className="levels__fill" style={{ width: `${percent}%` }} />
          </div>
          <span className="audio-test__value">{percent}%</span>
        </div>
        <p className="settings__hint">
          {t("audio_test_rms_rms_peak_peak", {
            rms: level.rms.toFixed(3),
            peak: level.peak.toFixed(3),
          })}
          {channel === "microphone" && micGain !== undefined && (
            <>
              {t("audio_test_gain_gain_applies_test_restarts", {
                gain: micGain.toFixed(1),
              })}
            </>
          )}
        </p>
        {silent && (
          <p className="settings__hint">
            {t("audio_test_no_signal_speak_closer_increase_microphone",
            )}
          </p>
        )}

        {channel === "microphone" && (
          <div className="audio-test__monitor">
            <label className="settings__field settings__field--row">
              <input
                type="checkbox"
                checked={monitoring}
                disabled={!running}
                onChange={() => void onToggleMonitor()}
              />
              <span>
                {monitorDelay !== null
                  ? t("audio_test_hear_my_voice_delay_s_delay", {
                      delay: (monitorDelay / 1000).toFixed(1),
                    })
                  : t("audio_test_hear_my_voice")}
              </span>
            </label>
            <p className="settings__hint">
              {t("audio_test_use_headphones_speakers_feedback_loops_back",
              )}
            </p>
          </div>
        )}
      </div>

      <div className="settings__actions">
        {running ? (
          <button
            type="button"
            className="settings__button"
            onClick={() => void onStop()}
          >
            {t("audio_test_stop_test")}
          </button>
        ) : (
          <button
            type="button"
            className="settings__button"
            disabled={sessionActive}
            onClick={() => void onStart()}
          >
            {t("audio_test_start_test")}
          </button>
        )}
      </div>

      {sessionActive && (
        <p className="settings__hint">
          {t("audio_test_session_active_end_test_audio")}
        </p>
      )}
      {error && (
        <p className="settings__status settings__status--error">{error}</p>
      )}
    </section>
  );
}
