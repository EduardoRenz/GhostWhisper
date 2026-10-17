import { useEffect, useRef } from "react";
import { speakerLabel, type TranscriptSegment } from "../../core/domain/transcript";

interface TranscriptViewProps {
  segments: TranscriptSegment[];
}

/** Lista rolável da transcrição ao vivo (ME/THEY). */
export function TranscriptView({ segments }: TranscriptViewProps) {
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // `scrollIntoView` pode não existir em ambientes de teste (jsdom).
    endRef.current?.scrollIntoView?.({ block: "end" });
  }, [segments.length]);

  return (
    <div className="transcript" data-interactive>
      {segments.length === 0 ? (
        <p className="transcript__empty">Nenhuma fala ainda…</p>
      ) : (
        segments.map((segment) => (
          <p
            key={segment.id}
            className={`transcript__line transcript__line--${segment.speaker.toLowerCase()}${
              segment.final ? "" : " transcript__line--interim"
            }`}
          >
            <span className="transcript__speaker">
              {speakerLabel(segment.speaker)}
            </span>
            <span className="transcript__text">{segment.text}</span>
          </p>
        ))
      )}
      <div ref={endRef} />
    </div>
  );
}
