import { memo } from "react";
import { useAudioLevel } from "../../core/application/useAudioLevel";
import {
  channelLabel,
  levelPercent,
  type SessionChannel,
} from "../../core/domain/transcript";

interface LevelMetersProps {
  channels: SessionChannel[];
  listening: boolean;
}

/**
 * Medidores de nível por canal ativo.
 *
 * A assinatura de `audio_level` (~10 eventos/s por canal) fica **aqui dentro**: se
 * o nível subisse até o `Overlay`, cada evento re-renderizaria a conversa inteira
 * (ISS-020). O componente é memoizado pelo mesmo motivo.
 */
export const LevelMeters = memo(function LevelMeters({
  channels,
  listening,
}: LevelMetersProps) {
  const levels = useAudioLevel();

  if (channels.length === 0) {
    return (
      <div className="levels-group" data-interactive>
        <div className="levels">
          <span className="levels__label">Inativo</span>
          <div className="levels__bar">
            <div className="levels__fill" style={{ width: "0%" }} />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="levels-group" data-interactive>
      {channels.map((channel) => {
        const percent = listening ? levelPercent(levels[channel]) : 0;
        return (
          <div className="levels" key={channel}>
            <span className="levels__label">{channelLabel(channel)}</span>
            <div className="levels__bar">
              <div className="levels__fill" style={{ width: `${percent}%` }} />
            </div>
          </div>
        );
      })}
    </div>
  );
});
