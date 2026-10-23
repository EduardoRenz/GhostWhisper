import { useCallback, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauriRuntime, onWindowFocus } from "../../adapters/ipc/client";
import { subscribe } from "../../adapters/ipc/events";
import {
  recordingPickerClose,
  recordingStartWith,
  recordingState,
  recordingSwitchTarget,
  recordingTargets,
} from "../../adapters/ipc/recording";
import { WindowHeader } from "../../components/WindowHeader";
import { MonitorIcon, VideoCameraIcon, WindowIcon } from "../../components/icons";
import { useTranslation } from "../../core/l10n";
import type { RecordingTarget } from "../../core/domain/recording";

/**
 * Seletor de alvo da gravação (BR-070).
 *
 * Aberto pelo clique secundário no botão de câmera. Lista a tela inteira, os
 * monitores e as janelas. Sem gravação, a escolha inicia; com gravação em
 * andamento, **troca o alvo** (fecha um segmento e abre outro no mesmo arquivo).
 */
export function RecordingPickerWindow() {
  const { t } = useTranslation();
  const [targets, setTargets] = useState<RecordingTarget[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [active, setActive] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const close = useCallback(() => {
    void recordingPickerClose();
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    const current = await recordingState();
    setActive(
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
    if (busy) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      if (active) {
        await recordingSwitchTarget(target);
      } else {
        await recordingStartWith(target);
      }
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
          title={t("recording_picker_record_window")}
          subtitle={
            active
              ? t("recording_picker_change_recording_target")
              : t("recording_picker_ghostwhisper_not_included_window_capture")
          }
          icon={<VideoCameraIcon size={18} />}
          onStartDrag={startDrag}
          onClose={close}
          closeLabel={t("recording_picker_close_picker")}
          dragLabel={t("recording_picker_move_picker")}
        />

        {active ? (
          <p className="recording-picker__notice">
            {t("recording_picker_recording_continues_same_file_new_section",
            )}
          </p>
        ) : null}

        {loading ? (
          <p className="recording-picker__loading">
            {t("recording_picker_looking_windows")}
          </p>
        ) : targets.length === 0 ? (
          <p className="recording-picker__loading">
            {t("recording_picker_no_windows_available_capture_requires_x11")}
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
                    disabled={busy}
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
                      {target.kind === "screen"
                        ? t("recording_picker_entire_screen")
                        : target.label}
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
          <span>
            {active
              ? t("recording_picker_click_change_target")
              : t("recording_picker_click_start_recording")}
          </span>
          <button type="button" className="recording-picker__cancel" onClick={close}>
            {t("recording_picker_cancel")}
          </button>
        </footer>
      </div>
    </div>
  );
}
