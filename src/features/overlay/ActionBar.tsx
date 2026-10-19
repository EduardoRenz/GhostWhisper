import { useEffect, useState } from "react";
import type { SessionChannel } from "../../core/domain/transcript";
import { SessionControls } from "../session/SessionControls";
import { SessionPill } from "../session/SessionPill";

interface ActionBarProps {
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
  /** A janela de Chat está visível. */
  chatOpen: boolean;
  onToggleChat: () => void;
  /** A janela de Histórico está visível. */
  historyOpen: boolean;
  onToggleHistory: () => void;
  /** Esconde o overlay. */
  onHide: () => void;
}

/**
 * Barra de ações do overlay (sempre visível).
 *
 * A conversa e o histórico vivem em **janelas próprias** (task 07/08): aqui só
 * ficam os controles da sessão e os toggles que escondem/mostram cada janela.
 */
export function ActionBar({
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
  chatOpen,
  onToggleChat,
  historyOpen,
  onToggleHistory,
  onHide,
}: ActionBarProps) {
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
        className={`overlay__button${chatOpen ? " overlay__button--on" : ""}`}
        data-interactive
        title="Mostra ou esconde a janela da conversa"
        onClick={onToggleChat}
      >
        Chat
      </button>

      <button
        type="button"
        className={`overlay__button${historyOpen ? " overlay__button--on" : ""}`}
        data-interactive
        title="Mostra ou esconde o histórico de sessões"
        onClick={onToggleHistory}
      >
        Histórico
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
        onClick={onHide}
      >
        Ocultar
      </button>
    </div>
  );
}
