import { useEffect, useState } from "react";
import { ALL_CHANNELS, type SessionChannel } from "../../core/domain/transcript";
import { IconButton } from "../../components/IconButton";
import {
  ChevronDownIcon,
  EyeIcon,
  EyeOffIcon,
  GearIcon,
  GripIcon,
  HistoryIcon,
  SparkleIcon,
} from "../../components/icons";
import { ShortcutBadges } from "../../components/ShortcutBadges";
import { PlayIcon } from "../../components/TransportIcons";
import { SessionPill } from "../session/SessionPill";
import { ChannelMenu } from "./ChannelMenu";
import { NotchTabs } from "./NotchTabs";

interface ActionBarProps {
  listening: boolean;
  paused: boolean;
  channels: SessionChannel[];
  /** Início da sessão (ms) — tempo exibido no timer. */
  startedAtMs: number | null;
  busy: boolean;
  onStart: (channels: SessionChannel[]) => void;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  onChannelsChange: (channels: SessionChannel[]) => void;
  onStartDrag: () => void;
  /** Atalho de iniciar/encerrar sessão (badges do botão verde). */
  startAccelerator: string;
  stealth: boolean;
  onToggleStealth: () => void;
  onOpenSettings: () => void;
  /** Abre/foca o painel na aba Transcrição. */
  onOpenTranscription: () => void;
  /** Abre/foca o painel na aba Resumo. */
  onOpenSummary: () => void;
  /** A janela de Sugestões está visível. */
  suggestionsOpen: boolean;
  onToggleSuggestions: () => void;
  /** A janela de Histórico está visível. */
  historyOpen: boolean;
  onToggleHistory: () => void;
  /** Esconde o overlay. */
  onHide: () => void;
}

/**
 * Barra de ações do overlay em **dois estados** (BR-032/task 08).
 *
 * - **Repouso** (sem sessão): marca, **"Começar a ouvir"** (esmeralda + badges),
 *   Histórico, ocultar e — **por último** — Configurações.
 * - **Ativo** (com sessão): marca, **switcher** (Transcrição + waveform ·
 *   play/pause · Histórico · Resumo), **timer**, canais, Sugestões, stealth,
 *   ocultar e Configurações (último).
 */
export function ActionBar({
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
  onStartDrag,
  startAccelerator,
  stealth,
  onToggleStealth,
  onOpenSettings,
  onOpenTranscription,
  onOpenSummary,
  suggestionsOpen,
  onToggleSuggestions,
  historyOpen,
  onToggleHistory,
  onHide,
}: ActionBarProps) {
  const [confirming, setConfirming] = useState(false);

  // Sessão encerrada (por qualquer caminho): a confirmação não fica pendurada.
  useEffect(() => {
    if (!listening) {
      setConfirming(false);
    }
  }, [listening]);

  return (
    <div
      className="overlay__bar"
      data-state={listening ? "active" : "idle"}
      data-interactive
    >
      <span
        className="overlay__drag"
        data-interactive
        title="Mover overlay"
        aria-label="Mover overlay"
        onPointerDown={onStartDrag}
      >
        <GripIcon size={16} />
      </span>

      <span className="overlay__brand-core">
        <span
          className={`overlay__brand-dot${
            listening && !paused ? " overlay__brand-dot--live" : ""
          }`}
          aria-hidden="true"
        />
        <span className="overlay__brand">GhostWhisper</span>
      </span>

      {!listening ? (
        <button
          type="button"
          className="overlay__start"
          data-interactive
          disabled={busy}
          title="Começa a captura com microfone e áudio do sistema"
          onClick={() => onStart(ALL_CHANNELS)}
        >
          <PlayIcon className="overlay__start-icon" />
          <span className="overlay__start-label">Começar a ouvir</span>
          <ShortcutBadges accelerator={startAccelerator} />
        </button>
      ) : (
        <>
          <NotchTabs
            listening={listening}
            paused={paused}
            busy={busy}
            historyOpen={historyOpen}
            onOpenTranscription={onOpenTranscription}
            onOpenSummary={onOpenSummary}
            onToggleHistory={onToggleHistory}
            onPause={onPause}
            onResume={onResume}
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

          <ChannelMenu
            channels={channels}
            busy={busy}
            onChannelsChange={onChannelsChange}
            onNewSession={onStart}
          />

          <IconButton
            label="Sugestões"
            title="Mostra ou esconde a janela de sugestões da IA"
            active={suggestionsOpen}
            pressed={suggestionsOpen}
            onClick={onToggleSuggestions}
          >
            <SparkleIcon />
          </IconButton>

          <IconButton
            label={stealth ? "Stealth ligado" : "Stealth desligado"}
            pressed={stealth}
            active={stealth}
            title="Modo stealth: sai da barra de tarefas. Não esconde o overlay do compartilhamento de tela (limite do Linux)."
            onClick={onToggleStealth}
          >
            {stealth ? <EyeIcon /> : <EyeOffIcon />}
          </IconButton>
        </>
      )}

      <div className="overlay__group overlay__group--end" data-interactive>
        {/* No repouso o Histórico fica aqui; no estado ativo ele vive no switcher. */}
        {!listening && (
          <IconButton
            label="Histórico"
            title="Mostra ou esconde o histórico de sessões"
            active={historyOpen}
            pressed={historyOpen}
            onClick={onToggleHistory}
          >
            <HistoryIcon />
          </IconButton>
        )}
        <IconButton
          label="Ocultar overlay"
          title="Esconde o overlay (atalho: mostrar/ocultar)"
          onClick={onHide}
        >
          <ChevronDownIcon />
        </IconButton>
        <IconButton label="Configurações" onClick={onOpenSettings}>
          <GearIcon />
        </IconButton>
      </div>
    </div>
  );
}
