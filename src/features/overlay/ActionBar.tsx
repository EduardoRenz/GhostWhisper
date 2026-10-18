import { useEffect, useState } from "react";
import type { SessionChannel } from "../../core/domain/transcript";
import { SessionControls } from "../session/SessionControls";
import { SessionPill } from "../session/SessionPill";
import type { Surface } from "../../adapters/ipc/overlay";

interface ActionBarProps {
  surface: Surface;
  onSurfaceChange: (surface: Surface) => void;
  stealth: boolean;
  onToggleStealth: () => void;
  listening: boolean;
  paused: boolean;
  channels: SessionChannel[];
  /** Início da sessão (ms) — tempo exibido na pílula de sessão. */
  startedAtMs: number | null;
  busy: boolean;
  onStart: (channels: SessionChannel[]) => void;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  onChannelsChange: (channels: SessionChannel[]) => void;
  onOpenSettings: () => void;
  onStartDrag: () => void;
}

/** Barra de ações do overlay (sempre visível). */
export function ActionBar({
  surface,
  onSurfaceChange,
  stealth,
  onToggleStealth,
  listening,
  paused,
  channels,
  startedAtMs,
  busy,
  onStart,
  onPause,
  onResume,
  onStop,
  onChannelsChange,
  onOpenSettings,
  onStartDrag,
}: ActionBarProps) {
  const expanded = surface !== "actionbar" && surface !== "hidden";
  const status = !listening ? "Parado" : paused ? "Pausado" : "Gravando";
  const tone = !listening ? "idle" : paused ? "paused" : "live";
  const [confirming, setConfirming] = useState(false);

  // Sessão encerrada (por qualquer caminho): a confirmação não fica pendurada.
  useEffect(() => {
    if (!listening) {
      setConfirming(false);
    }
  }, [listening]);

  return (
    <div className="overlay__bar" data-interactive>
      <span
        className="overlay__drag"
        data-interactive
        title="Mover overlay"
        aria-label="Mover overlay"
        onPointerDown={onStartDrag}
      >
        ⋮⋮
      </span>
      <span className="overlay__brand">GhostWhisper</span>
      <span
        className={`overlay__status overlay__status--${tone}`}
        data-interactive
      >
        <span className={`overlay__dot overlay__dot--${tone}`} aria-hidden="true" />
        {status}
      </span>

      <SessionControls
        listening={listening}
        paused={paused}
        channels={channels}
        busy={busy}
        onStart={onStart}
        onPause={onPause}
        onResume={onResume}
        onRequestStop={() => setConfirming(true)}
        onChannelsChange={onChannelsChange}
      />

      <SessionPill
        startedAtMs={listening ? startedAtMs : null}
        confirming={confirming}
        busy={busy}
        onRequestStop={() => setConfirming(true)}
        onCancelStop={() => setConfirming(false)}
        onStop={() => {
          setConfirming(false);
          onStop();
        }}
      />

      <button
        type="button"
        className="overlay__button"
        data-interactive
        onClick={() => onSurfaceChange(expanded ? "actionbar" : "expanded")}
      >
        {expanded ? "Recolher" : "Expandir"}
      </button>

      <button
        type="button"
        className="overlay__button"
        data-interactive
        title="Modo stealth: sai da barra de tarefas e do título. Não esconde o overlay do compartilhamento de tela (limite do Linux)."
        onClick={onToggleStealth}
      >
        {stealth ? "Stealth: on" : "Stealth: off"}
      </button>

      <button
        type="button"
        className="overlay__button"
        data-interactive
        onClick={onOpenSettings}
      >
        Ajustes
      </button>

      <button
        type="button"
        className="overlay__button"
        data-interactive
        onClick={() => onSurfaceChange("hidden")}
      >
        Ocultar
      </button>
    </div>
  );
}
