import { useEffect, useState } from "react";
import type { Suggestion } from "../../core/domain/assistant";
import { formatMessageClock } from "../../core/domain/transcript";
import {
  NO_SUGGESTION_TITLE,
  parseSuggestionMeta,
  stripSuggestionMeta,
  suggestionKind,
  suggestionTitle,
} from "../../core/domain/suggestion";
import { AgentBadge } from "../../components/AgentBadge";
import { LlmSourceBadge } from "../../components/LlmSourceBadge";
import { CopyIcon, PinIcon, SparkleIcon } from "../../components/icons";
import { Pill } from "../../components/Pill";
import { MarkdownView } from "../../components/MarkdownView";
import type { LlmSourceUsage } from "../../core/application/useLlmSource";
import { useTranslation, type TranslationKey, type TranslationParams } from "../../core/l10n";
import {
  speakerLabelKey,
  suggestionKindKey,
} from "../../core/l10n/labels";

interface SuggestionCardProps {
  /** Sugestão completa (Markdown). */
  current: Suggestion | null;
  /** Resposta curta da mesma geração (fase `quick`). */
  quick: Suggestion | null;
  /** Copia o texto da sugestão. */
  onCopy: () => void;
  /** Fixa a sugestão na conversa. */
  onPin: () => void;
  /** `true` quando a sugestão acabou de ser fixada. */
  pinned: boolean;
  /** `true` quando o texto acabou de ser copiado. */
  copied: boolean;
  /** `true` enquanto a resposta completa da geração atual está pendente. */
  awaitingFull: boolean;
  /** Início da sessão (hora de parede no rótulo manual — BR-040). */
  startedAtMs?: number | null;
  /** Ações extras (ex.: "Fixar"/"Copiar" em texto). */
  children?: React.ReactNode;
  /** Modelo que respondeu a sugestão (BR-051). */
  llmSource?: LlmSourceUsage | null;
}

/** Tempo relativo curto (“Há 12s”, “Há 2min”). */
function relativeTime(
  ms: number,
  t: (key: TranslationKey, params?: TranslationParams) => string,
): string {
  const seconds = Math.max(0, Math.round(ms / 1000));
  if (seconds < 60) {
    return t("suggestions_seconds_s_ago", { seconds });
  }
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) {
    return t("suggestions_minutes_min_ago", { minutes });
  }
  return t("suggestions_hours_h_ago", { hours: Math.round(minutes / 60) });
}

/**
 * Cartão da sugestão atual (BR-038).
 *
 * Mostra a tag de momento, o **título derivativo**, a pill do agente, o bloco
 * de **resposta rápida** e o bloco de **resposta completa** (com o chip de
 * metadados). A resposta rápida é efêmera: some quando chega outra geração.
 */
export function SuggestionCard({
  current,
  quick,
  onCopy,
  onPin,
  pinned,
  copied,
  awaitingFull,
  startedAtMs,
  children,
  llmSource,
}: SuggestionCardProps) {
  const { t } = useTranslation();
  const [receivedAt, setReceivedAt] = useState<number | null>(null);
  const [now, setNow] = useState(() => Date.now());

  // A contagem recomeça a cada nova geração (a rápida e a completa da mesma
  // geração compartilham `generation_id`).
  useEffect(() => {
    setReceivedAt(Date.now());
    setNow(Date.now());
  }, [current?.generation_id, quick?.generation_id]);

  // Relógio do rótulo (1 s).
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, []);

  const source = current ?? quick;
  if (!source) {
    return null;
  }

  // Título: o **gerado pelo agente** (`Subject:`) quando existe; senão o
  // derivado do texto (que ignora seções genéricas) — BR-038.
  const aiTitle = current?.title?.trim() || quick?.title?.trim() || "";
  const fullTitle = current ? suggestionTitle(current.text) : NO_SUGGESTION_TITLE;
  const quickTitle = quick ? suggestionTitle(quick.text) : NO_SUGGESTION_TITLE;
  const derivedTitle =
    fullTitle !== NO_SUGGESTION_TITLE ? fullTitle : quickTitle;
  const title = aiTitle || derivedTitle || NO_SUGGESTION_TITLE;
  const displayTitle =
    title === NO_SUGGESTION_TITLE
      ? t("suggestions_default_title")
      : title;
  // Com bloco de código, os metadados vão para o caption: some do corpo.
  const hasCode = Boolean(current?.text.includes("```"));
  const body = current && hasCode ? stripSuggestionMeta(current.text) : current?.text;
  const meta = parseSuggestionMeta(current?.text);
  const agent = {
    name: source.mode_name,
    icon: source.mode_icon,
    color: source.mode_color,
  };
  const manual = (current?.trigger ?? quick?.trigger) === "manual";
  const origin = current?.source ?? quick?.source ?? null;

  return (
    <div className="suggestion-card">
      <div className="suggestion-card__head">
        <span className="suggestion-card__tag">
          <SparkleIcon size={11} />
          {manual ? t("suggestions_manual_suggestion") : t("suggestions_current_suggestion")}
          {receivedAt !== null && (
            <span className="suggestion-card__age">
              · {relativeTime(now - receivedAt, t)}
            </span>
          )}
        </span>
        <div className="suggestion-card__badges">
          <AgentBadge mode={agent} />
          <LlmSourceBadge usage={llmSource ?? null} />
        </div>
      </div>

      <h2 className="suggestion-card__title">{displayTitle}</h2>

      {origin && (
        <p className="suggestion-card__source">
          {t("suggestions_manual_speaker_time", {
            speaker: t(speakerLabelKey(origin.speaker)),
            time: formatMessageClock(origin.at_ms, startedAtMs),
          })}
        </p>
      )}

      {children}

      {quick && (
        <section className="suggestion-block suggestion-block--quick">
          <span className="suggestion-block__label">{t("suggestions_quick_response")}</span>
          <p className="suggestion-block__text">{quick.text}</p>
        </section>
      )}

      {quick && !current && awaitingFull && (
        <p className="suggestion-block__pending" role="status">
          <span className="suggestion-block__dot" aria-hidden="true" />
          {t("suggestions_generating_full_response")}
        </p>
      )}

      {current && (
        <section className="suggestion-block suggestion-block--full">
          <div className="suggestion-block__head">
            <span className="suggestion-block__label">{t("suggestions_full_response")}</span>
            <Pill className="suggestion-chip">
              {t("suggestions_markdown_kind", {
                kind: t(suggestionKindKey(suggestionKind(current.text))),
              })}
            </Pill>
          </div>
          <MarkdownView markdown={body ?? ""} meta={meta} />
        </section>
      )}

      <div className="suggestion-card__actions">
        <button
          type="button"
          className="icon-button"
          onClick={onPin}
          disabled={!current}
          title={pinned ? t("suggestions_suggestion_pinned") : t("suggestions_pin_conversation")}
          aria-label={pinned ? t("suggestions_suggestion_pinned") : t("suggestions_pin_conversation")}
        >
          <PinIcon size={15} />
        </button>
        <button
          type="button"
          className="icon-button"
          onClick={onCopy}
          disabled={!current}
          title={copied ? t("suggestions_copied") : t("suggestions_copy")}
          aria-label={copied ? t("suggestions_copied") : t("suggestions_copy")}
        >
          <CopyIcon size={15} />
        </button>
      </div>
    </div>
  );
}
