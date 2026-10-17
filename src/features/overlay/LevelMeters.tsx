import {
  channelLabel,
  levelPercent,
  type AudioLevel,
  type SessionChannel,
} from "../../core/domain/transcript";

interface LevelMetersProps {
  levels: Record<SessionChannel, AudioLevel>;
  channels: SessionChannel[];
  listening: boolean;
}

/** Medidores de nível por canal ativo. */
export function LevelMeters({ levels, channels, listening }: LevelMetersProps) {
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
}
