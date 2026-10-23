import { useEffect, useState } from "react";
import logoUrl from "../../assets/logo.png";
import type {
  RecordingCapabilities,
  RecordingState,
} from "../../core/domain/recording";
import { ALL_CHANNELS, type SessionChannel } from "../../core/domain/transcript";
import { IconButton } from "../../components/IconButton";
import {
  CollapseIcon,
  EyeIcon,
  EyeOffIcon,
  GearIcon,
  HistoryIcon,
  SearchIcon,
} from "../../components/icons";
import { DragHandle } from "../../components/DragHandle";
import { ShortcutBadges } from "../../components/ShortcutBadges";
import { PlayIcon } from "../../components/TransportIcons";
import { useTranslation } from "../../core/l10n";
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
  /** Abre o menu de canais na janela flutuante (BR-072/ISS-063). */
  onOpenChannelMenu?: (anchor: DOMRect) => void;
  onStartDrag: () => void;
  /** Atalho de iniciar/encerrar sessão (badges do botão verde). */
  startAccelerator: string;
  /** Atalho da barra de pesquisa (badge da lupa). */
  askAccelerator: string;
  stealth: boolean;
  onToggleStealth: () => void;
  onOpenSettings: () => void;
  /** Abre/foca o painel na aba Transcrição. */
  onOpenTranscription: () => void;
  /** Abre/foca o painel na aba Resumo. */
  onOpenSummary: () => void;
  /** Abre a barra de pesquisa (janela `ask`) — BR-052. */
  onOpenAsk: () => void;
  /** A janela de Sugestões está visível. */
  suggestionsOpen: boolean;
  onToggleSuggestions: () => void;
  /** A janela de Histórico está visível. */
  historyOpen: boolean;
  onToggleHistory: () => void;
  /** Esconde o overlay. */
  onHide: () => void;
  /** Gravação da sessão (BR-065). */
  recording: RecordingState;
  recordingBusy: boolean;
  recordingCapabilities: RecordingCapabilities | null;
  onToggleRecording: () => void;
  onPickRecordingTarget: () => void;
}

/**
 * Barra de ações do overlay em **dois estados** (BR-032/task 08).
 *
 * - **Repouso** (sem sessão): marca, **lupa** (barra de pesquisa),
 *   **"Começar a ouvir"** (esmeralda + badges), ocultar, Histórico e — **por
 *   último** — Configurações.
 * - **Ativo** (com sessão): marca, **lupa**, **switcher** (Transcrição +
 *   waveform · play/pause · Resumo · Sugestões), **timer**, canais, stealth,
 *   **ocultar**, **Histórico** e Configurações (último).
 *
 * A **lupa** existe nos dois estados (BR-052): a barra de pesquisa não depende
 * de haver transcrição.
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
  onOpenChannelMenu,
  onStartDrag,
  startAccelerator,
  askAccelerator,
  stealth,
  onToggleStealth,
  onOpenSettings,
  onOpenTranscription,
  onOpenSummary,
  onOpenAsk,
  suggestionsOpen,
  onToggleSuggestions,
  historyOpen,
  onToggleHistory,
  onHide,
  recording,
  recordingBusy,
  recordingCapabilities,
  onToggleRecording,
  onPickRecordingTarget,
}: ActionBarProps) {
  const { t } = useTranslation();
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
      <DragHandle
        className="overlay__drag"
        label={t("action_bar_move_overlay")}
        onStartDrag={onStartDrag}
      />

      <span className="overlay__brand-core">
        <span className="overlay__brand-mark">
          <img
            className="overlay__brand-logo"
            src={logoUrl}
            alt=""
            aria-hidden="true"
          />
          <span
            className={`overlay__brand-dot${
              listening && !paused ? " overlay__brand-dot--live" : ""
            }`}
            aria-hidden="true"
          />
        </span>
        <span className="overlay__brand">GhostWhisper</span>
      </span>

      {/* Lupa: abre a barra de pesquisa (BR-052), com ou sem transcrição. */}
      <button
        type="button"
        className="overlay__search"
        data-interactive
        disabled={busy}
        title={t("action_bar_ask_ghostwhisper_about_screen_audio_accelerator", {
          accelerator: askAccelerator,
        })}
        aria-label={t("action_bar_ask_ghostwhisper")}
        onClick={onOpenAsk}
      >
        <SearchIcon className="overlay__search-icon" size={16} />
        <ShortcutBadges accelerator={askAccelerator} />
      </button>

      {!listening ? (
        <button
          type="button"
          className="overlay__start"
          data-interactive
          disabled={busy}
          title={t("action_bar_starts_capturing_microphone_system_audio")}
          onClick={() => onStart(ALL_CHANNELS)}
        >
          <PlayIcon className="overlay__start-icon" />
          <span className="overlay__start-label">{t("action_bar_start_listening")}</span>
          <ShortcutBadges accelerator={startAccelerator} />
        </button>
      ) : (
        <>
          <NotchTabs
            listening={listening}
            paused={paused}
            busy={busy}
            suggestionsOpen={suggestionsOpen}
            onOpenTranscription={onOpenTranscription}
            onOpenSummary={onOpenSummary}
            onToggleSuggestions={onToggleSuggestions}
            onPause={onPause}
            onResume={onResume}
            recording={recording}
            recordingBusy={recordingBusy}
            recordingCapabilities={recordingCapabilities}
            onToggleRecording={onToggleRecording}
            onPickRecordingTarget={onPickRecordingTarget}
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

          <ChannelMenu channels={channels} onOpenMenu={onOpenChannelMenu} />

          <IconButton
            label={stealth ? t("action_bar_stealth") : t("action_bar_stealth_off")}
            pressed={stealth}
            active={stealth}
            title={t("action_bar_stealth_mode_hides_taskbar_not_hide",
            )}
            onClick={onToggleStealth}
          >
            {stealth ? <EyeIcon /> : <EyeOffIcon />}
          </IconButton>
        </>
      )}

      <div className="overlay__group overlay__group--end" data-interactive>
        <IconButton
          label={t("action_bar_hide_overlay")}
          title={t("action_bar_hides_overlay_shortcut_show_hide")}
          onClick={onHide}
        >
          <CollapseIcon />
        </IconButton>
        <IconButton
          label={t("action_bar_history")}
          title={t("action_bar_shows_hides_session_history")}
          active={historyOpen}
          pressed={historyOpen}
          onClick={onToggleHistory}
        >
          <HistoryIcon />
        </IconButton>
        <IconButton label={t("action_bar_settings")} onClick={onOpenSettings}>
          <GearIcon />
        </IconButton>
      </div>
    </div>
  );
}
