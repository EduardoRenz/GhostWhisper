import type { SessionChannel } from "../../core/domain/transcript";
import { SessionControls } from "../session/SessionControls";
import type { Surface } from "../../adapters/ipc/overlay";

interface ActionBarProps {
  surface: Surface;
  onSurfaceChange: (surface: Surface) => void;
  stealth: boolean;
  onToggleStealth: () => void;
  listening: boolean;
  busy: boolean;
  onStart: (channels: SessionChannel[]) => void;
  onStop: () => void;
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
  busy,
  onStart,
  onStop,
  onOpenSettings,
  onStartDrag,
}: ActionBarProps) {
  const expanded = surface === "expanded";

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
        className={`overlay__status overlay__status--${listening ? "live" : "idle"}`}
        data-interactive
      >
        {listening ? "● gravando" : "○ parado"}
      </span>

      <SessionControls
        listening={listening}
        busy={busy}
        onStart={onStart}
        onStop={onStop}
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
