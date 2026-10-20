import { useEffect, useRef, useState } from "react";
import { MicIcon, MicOffIcon } from "../../components/icons";
import type { SessionChannel } from "../../core/domain/transcript";

type ChannelChoice = "both" | SessionChannel;

const CHOICES: { value: ChannelChoice; label: string }[] = [
  { value: "both", label: "Ambos" },
  { value: "microphone", label: "Microfone" },
  { value: "system", label: "Sistema" },
];

/** Converte a lista de canais na opção do seletor. */
const choiceOf = (channels: SessionChannel[]): ChannelChoice => {
  const hasMic = channels.includes("microphone");
  const hasSystem = channels.includes("system");
  if (hasMic && hasSystem) return "both";
  return hasSystem ? "system" : "microphone";
};

const channelsOf = (choice: ChannelChoice): SessionChannel[] =>
  choice === "both" ? ["microphone", "system"] : [choice];

interface ChannelMenuProps {
  /** Canais captados agora. */
  channels: SessionChannel[];
  busy: boolean;
  /** Troca os canais **sem** encerrar a sessão (BR-019). */
  onChannelsChange: (channels: SessionChannel[]) => void;
  /** Começa a capturar nesta sessão com os canais escolhidos (mesmo id). */
  onNewSession: (channels: SessionChannel[]) => void;
}

/**
 * Canais de áudio do notch ativo (BR-019/task 08).
 *
 * O botão mostra `mic` (capta microfone) ou `mic-off` (só sistema) e abre um
 * popover com as três opções + **Nova sessão**. Substitui o antigo `<select>` +
 * botão de texto, mantendo a troca de canais ao vivo.
 */
export function ChannelMenu({
  channels,
  busy,
  onChannelsChange,
  onNewSession,
}: ChannelMenuProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const choice = choiceOf(channels);
  const hasMic = channels.includes("microphone");

  useEffect(() => {
    if (!open) {
      return;
    }
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  const select = (next: ChannelChoice) => {
    onChannelsChange(channelsOf(next));
    setOpen(false);
  };

  return (
    <div className="channel-menu" ref={rootRef} data-interactive>
      <button
        type="button"
        className="icon-button"
        aria-label="Canais de áudio"
        aria-haspopup="menu"
        aria-expanded={open}
        title="Canais de áudio (Ambos / Microfone / Sistema)"
        onClick={() => setOpen((previous) => !previous)}
      >
        {hasMic ? <MicIcon /> : <MicOffIcon />}
      </button>

      {open && (
        <div className="channel-menu__popover" role="menu" data-interactive>
          <span className="channel-menu__title">Canais</span>
          {CHOICES.map((option) => (
            <button
              key={option.value}
              type="button"
              role="menuitemradio"
              aria-checked={choice === option.value}
              className={`channel-menu__item${
                choice === option.value ? " channel-menu__item--on" : ""
              }`}
              onClick={() => select(option.value)}
            >
              {option.label}
            </button>
          ))}
          <div className="channel-menu__divider" />
          <button
            type="button"
            role="menuitem"
            className="channel-menu__item"
            disabled={busy}
            onClick={() => {
              onNewSession(channelsOf(choice));
              setOpen(false);
            }}
          >
            Começar a capturar
          </button>
        </div>
      )}
    </div>
  );
}
