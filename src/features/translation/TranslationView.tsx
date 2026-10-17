import { copyText } from "../../core/application/clipboard";
import type { TranslationMap } from "../../core/application/useTranslation";
import type { TranscriptSegment } from "../../core/domain/transcript";

interface TranslationViewProps {
  segments: TranscriptSegment[];
  translations: TranslationMap;
  targetLanguage: string;
}

/** Tradução ao vivo lado a lado com o texto original. */
export function TranslationView({
  segments,
  translations,
  targetLanguage,
}: TranslationViewProps) {
  const items = segments.filter((segment) => translations[segment.id]);

  if (items.length === 0) {
    return (
      <div className="translation translation--empty" data-interactive>
        Tradução ({targetLanguage}) aparecerá aqui…
      </div>
    );
  }

  return (
    <div className="translation" data-interactive>
      {items.map((segment) => {
        const translated = translations[segment.id] ?? "";
        return (
          <p key={segment.id} className="translation__line">
            <span className="translation__source">{segment.text}</span>
            <span className="translation__text">{translated}</span>
            <button
              type="button"
              className="overlay__button translation__copy"
              data-interactive
              onClick={() => void copyText(translated)}
            >
              Copiar
            </button>
          </p>
        );
      })}
    </div>
  );
}
