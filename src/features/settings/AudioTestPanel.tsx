import { useEffect, useState } from "react";
import {
  listMicrophones,
  listSystemSources,
  startMicrophone,
  startSystemAudio,
  stopMicrophone,
  stopSystemAudio,
  type AudioDevice,
} from "../../adapters/ipc/audio";
import { useAudioLevel } from "../../core/application/useAudioLevel";
import { useSessionState } from "../../core/application/useSessionState";
import {
  channelLabel,
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
  const [channel, setChannel] = useState<SessionChannel>("microphone");
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const levels = useAudioLevel();
  const { state } = useSessionState();
  const sessionActive = state.listening;

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

  // Nunca deixa uma captura de teste pendurada ao sair da página.
  useEffect(
    () => () => {
      void stopMicrophone();
      void stopSystemAudio();
    },
    [],
  );

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
      <span className="settings__group-title">Teste de áudio</span>
      <p className="settings__hint">
        Confira o microfone (ou o áudio do sistema) antes de iniciar uma sessão.
        Fale e veja a barra reagir.
      </p>

      <label className="settings__field">
        <span>Canal</span>
        <select
          value={channel}
          disabled={running}
          onChange={(event) =>
            setChannel(event.target.value as SessionChannel)
          }
        >
          <option value="microphone">Microfone</option>
          <option value="system">Áudio do sistema</option>
        </select>
      </label>

      <label className="settings__field">
        <span>Dispositivo</span>
        <select
          value={deviceId ?? ""}
          disabled={running || devices.length === 0}
          onChange={(event) => setDeviceId(event.target.value || null)}
        >
          {devices.length === 0 && <option value="">(nenhum encontrado)</option>}
          {devices.map((device) => (
            <option key={device.id} value={device.id}>
              {device.name}
              {device.is_default ? " (padrão)" : ""}
            </option>
          ))}
        </select>
      </label>

      <div className="audio-test">
        <div className="levels">
          <span className="levels__label">{channelLabel(channel)}</span>
          <div className="levels__bar">
            <div className="levels__fill" style={{ width: `${percent}%` }} />
          </div>
          <span className="audio-test__value">{percent}%</span>
        </div>
        <p className="settings__hint">
          RMS {level.rms.toFixed(3)} · pico {level.peak.toFixed(3)}
          {channel === "microphone" && micGain !== undefined && (
            <> · ganho {micGain.toFixed(1)}× (aplica-se ao reiniciar o teste)</>
          )}
        </p>
        {silent && (
          <p className="settings__hint">
            Sem sinal: fale mais perto, aumente a “Sensibilidade do microfone”
            acima ou o volume de entrada do sistema. O ruído de fundo abaixo da
            sensibilidade é silenciado de propósito para não virar transcrição
            falsa.
          </p>
        )}
      </div>

      <div className="settings__actions">
        {running ? (
          <button
            type="button"
            className="settings__button"
            onClick={() => void onStop()}
          >
            Parar teste
          </button>
        ) : (
          <button
            type="button"
            className="settings__button"
            disabled={sessionActive}
            onClick={() => void onStart()}
          >
            Iniciar teste
          </button>
        )}
      </div>

      {sessionActive && (
        <p className="settings__hint">
          Há uma sessão ativa — encerre-a para testar o áudio.
        </p>
      )}
      {error && (
        <p className="settings__status settings__status--error">{error}</p>
      )}
    </section>
  );
}
