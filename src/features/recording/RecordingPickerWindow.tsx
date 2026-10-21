import { useCallback, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauriRuntime, onWindowFocus } from "../../adapters/ipc/client";
import { subscribe } from "../../adapters/ipc/events";
import {
  recordingPickerClose,
  recordingStartWith,
  recordingState,
  recordingTargets,
} from "../../adapters/ipc/recording";
import { WindowHeader } from "../../components/WindowHeader";
import { MonitorIcon, VideoCameraIcon, WindowIcon } from "../../components/icons";
import type { RecordingTarget } from "../../core/domain/recording";

/**
 * Seletor de alvo da gravação (BR-070).
 *
 * Aberto pelo clique secundário no botão de câmera. Lista a tela inteira e as
 * janelas abertas; a escolha inicia a gravação. Como o alvo é congelado na
 * sessão, o seletor fica bloqueado enquanto já existe gravação em andamento.
 */
export function RecordingPickerWindow() {
  const [targets, setTargets] = useState<RecordingTarget[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [blocked, setBlocked] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const close = useCallback(() => {
    void recordingPickerClose();
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    const current = await recordingState();
    setBlocked(
      current
        ? current.status === "recording" || current.status === "paused"
        : false,
    );
    const list = await recordingTargets();
    setTargets(list ?? []);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
    const unlisteners: (() => void)[] = [];
    const keep = (stop: () => void) => {
      unlisteners.push(stop);
    };
    // A janela é criada no boot; ao abrir, o core avisa para reler os alvos.
    void subscribe("recording_picker_open", () => {
      void load();
    }).then(keep);
    void onWindowFocus((focused) => {
      if (focused) {
        void load();
      }
    }).then(keep);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      for (const stop of unlisteners) {
        stop();
      }
    };
  }, [close, load]);

  const startDrag = () => {
    if (isTauriRuntime()) {
      void getCurrentWindow().startDragging();
    }
  };

  const pick = async (target: RecordingTarget) => {
    if (busy || blocked) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await recordingStartWith(target);
      await recordingPickerClose();
    } catch (caught) {
      setError(String(caught));
      setBusy(false);
    }
  };

  return (
    <div className="recording-picker" data-interactive>
      <div className="recording-picker__card">
        <WindowHeader
          className="recording-picker__header"
          title="Gravar janela"
          subtitle="O GhostWhisper não entra na captura da janela"
          icon={<VideoCameraIcon size={18} />}
          onStartDrag={startDrag}
          onClose={close}
          closeLabel="Fechar seletor"
          dragLabel="Mover seletor"
        />

        {blocked ? (
          <p className="recording-picker__notice">
            Já existe uma gravação em andamento. Pare a atual para escolher
            outro alvo.
          </p>
        ) : null}

        {loading ? (
          <p className="recording-picker__loading">Procurando janelas…</p>
        ) : targets.length === 0 ? (
          <p className="recording-picker__loading">
            Nenhuma janela disponível. A captura exige X11/XWayland.
          </p>
        ) : (
          <ul className="recording-picker__list">
            {targets.map((target) => {
              const key =
                target.kind === "window" ? `w-${target.id}` : "screen";
              return (
                <li key={key}>
                  <button
                    type="button"
                    className="recording-picker__item"
                    disabled={busy || blocked}
                    onClick={() => void pick(target)}
                  >
                    <span className="recording-picker__item-icon">
                      {target.kind === "window" ? (
                        <WindowIcon size={16} />
                      ) : (
                        <MonitorIcon size={16} />
                      )}
                    </span>
                    <span className="recording-picker__item-label">
                      {target.label}
                    </span>
                    {target.width > 0 && target.height > 0 ? (
                      <span className="recording-picker__item-size">
                        {target.width}×{target.height}
                      </span>
                    ) : null}
                  </button>
                </li>
              );
            })}
          </ul>
        )}

        {error ? <p className="recording-picker__error">{error}</p> : null}

        <footer className="recording-picker__footer">
          <span>Clique para iniciar a gravação</span>
          <button type="button" className="recording-picker__cancel" onClick={close}>
            Cancelar
          </button>
        </footer>
      </div>
    </div>
  );
}
