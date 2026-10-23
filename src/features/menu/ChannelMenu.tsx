import { closeMenu } from "../../adapters/ipc/overlay";
import { useSessionState } from "../../core/application/useSessionState";
import { useTranslation, type TranslationKey } from "../../core/l10n";
import type { SessionChannel } from "../../core/domain/transcript";

type ChannelChoice = "both" | SessionChannel;

const CHOICES: ChannelChoice[] = ["both", "microphone", "system"];

const CHOICE_KEYS: Record<ChannelChoice, TranslationKey> = {
  both: "common_both",
  microphone: "common_microphone",
  system: "common_system",
};

const choiceOf = (channels: SessionChannel[]): ChannelChoice => {
  const hasMic = channels.includes("microphone");
  const hasSystem = channels.includes("system");
  if (hasMic && hasSystem) return "both";
  return hasSystem ? "system" : "microphone";
};

const channelsOf = (choice: ChannelChoice): SessionChannel[] =>
  choice === "both" ? ["microphone", "system"] : [choice];

/**
 * Lista de canais dentro da janela flutuante (BR-019/BR-072/ISS-063).
 *
 * Troca os canais ao vivo e inicia uma nova sessão com a escolha atual.
 */
export function ChannelMenu() {
  const { t } = useTranslation();
  const { state, busy, setChannels, start } = useSessionState();
  const choice = choiceOf(state.channels);

  return (
    <div
      className="menu-list"
      role="menu"
      aria-label={t("channel_menu_channels")}
    >
      <span className="channel-menu__title">{t("channel_menu_channels")}</span>
      {CHOICES.map((value) => (
        <button
          key={value}
          type="button"
          role="menuitemradio"
          aria-checked={choice === value}
          className={`channel-menu__item${
            choice === value ? " channel-menu__item--on" : ""
          }`}
          onClick={() => {
            void setChannels(channelsOf(value)).finally(
              () => void closeMenu(),
            );
          }}
        >
          {t(CHOICE_KEYS[value])}
        </button>
      ))}
      <div className="channel-menu__divider" />
      <button
        type="button"
        role="menuitem"
        className="channel-menu__item"
        disabled={busy}
        onClick={() => {
          void start(channelsOf(choice)).finally(() => void closeMenu());
        }}
      >
        {t("channel_menu_start_capturing")}
      </button>
    </div>
  );
}
