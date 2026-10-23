import { useRef } from "react";
import { MicIcon, MicOffIcon } from "../../components/icons";
import { useTranslation } from "../../core/l10n";
import type { SessionChannel } from "../../core/domain/transcript";

interface ChannelMenuProps {
  /** Canais captados agora (define o ícone do botão). */
  channels: SessionChannel[];
  /** Abre o menu de canais na janela flutuante (CSS px relativo à janela). */
  onOpenMenu?: (anchor: DOMRect) => void;
}

/**
 * Botão de canais de áudio do notch ativo (BR-019/task 08).
 *
 * O ícone mostra `mic` (capta microfone) ou `mic-off` (só sistema). As opções
 * (Ambos/Mic/Sistema + Nova sessão) vivem no menu flutuante em janela própria
 * (BR-072/ISS-063), para o notch não mudar de tamanho.
 */
export function ChannelMenu({ channels, onOpenMenu }: ChannelMenuProps) {
  const { t } = useTranslation();
  const buttonRef = useRef<HTMLButtonElement>(null);
  const hasMic = channels.includes("microphone");

  return (
    <div className="channel-menu" data-interactive>
      <button
        ref={buttonRef}
        type="button"
        className="icon-button"
        aria-label={t("channel_menu_audio_channels")}
        aria-haspopup="menu"
        onClick={() => {
          const rect = buttonRef.current?.getBoundingClientRect();
          if (rect) {
            onOpenMenu?.(rect);
          }
        }}
      >
        {hasMic ? <MicIcon /> : <MicOffIcon />}
      </button>
    </div>
  );
}
