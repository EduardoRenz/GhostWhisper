import type { SessionChannel } from "../../core/domain/transcript";
import { SessionControls } from "../session/SessionControls";
import type { Surface } from "../../adapters/ipc/overlay";

interface ActionBarProps {
  surface: Surface;
  onSurfaceChange: (surface: Surface) => void;
  stealth: boolean;
  onToggleStealth: () => void;
  listening: boolean;
  paused: boolean;
  channels: SessionChannel[];
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
  const status = !listening ? "○ parado" : paused ? "❚❚ pausado" : "● gravando";

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
        className={`overlay__status overlay__status--${
          paused ? "paused" : listening ? "live" : "idle"
        }`}
        data-interactive
      >
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
        onStop={onStop}
        onChannelsChange={onChannelsChange}
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
