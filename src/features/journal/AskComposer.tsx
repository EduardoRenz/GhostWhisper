import { useState } from "react";
import { CameraIcon, SendIcon } from "../../components/icons";
import { useTranslation } from "../../core/l10n";

interface AskComposerProps {
  /** Perguntas prontas (chips). */
  presets: string[];
  /** Miniatura do print pendente (data URL) — `null` sem anexo. */
  attachment: string | null;
  /** Uma pergunta/captura está em curso. */
  busy: boolean;
  /** Aviso (ex.: captura indisponível). */
  error: string | null;
  /** Envia a pergunta (com ou sem print). */
  onSend: (question: string, source: "search" | "preset") => void;
  /** Captura a tela e **anexa** (não envia). */
  onCapture: () => void;
  /** Descarta o print anexado. */
  onRemoveAttachment: () => void;
}

/**
 * Composer **compacto** da aba Sessão (BR-053).
 *
 * Uma linha: câmera + campo + enviar; as perguntas prontas vêm como chips
 * abaixo. `Enter` envia. Com um print anexado o texto é **opcional** — a IA
 * decide o que fazer com a imagem. Capturar nunca envia: só anexa.
 */
export function AskComposer({
  presets,
  attachment,
  busy,
  error,
  onSend,
  onCapture,
  onRemoveAttachment,
}: AskComposerProps) {
  const { t } = useTranslation();
  const [value, setValue] = useState("");

  const send = (text: string, source: "search" | "preset" = "search") => {
    const question = text.trim();
    // Sem texto só é permitido **com print anexado** (a IA decide).
    if (busy || (question === "" && attachment === null)) {
      return;
    }
    onSend(question, source);
    setValue("");
  };

  return (
    <div className="composer" data-interactive>
      <div className="composer__row">
        <button
          type="button"
          className="composer__button composer__button--camera"
          title={
            attachment
              ? t("journal_screenshot_already_attached_confirm_question_send",
                )
              : t("journal_captures_screen_attaches_question_alt_c")
          }
          aria-label={t("journal_capture_screen")}
          disabled={busy || attachment !== null}
          onClick={onCapture}
        >
          <CameraIcon size={16} />
        </button>

        <input
          className="composer__input"
          type="text"
          value={value}
          placeholder={t("journal_ask_about_meeting_screen")}
          aria-label={t("journal_question_ai")}
          disabled={busy}
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              send(value);
            }
          }}
        />

        <button
          type="button"
          className="composer__button composer__button--send"
          title={t("journal_ask")}
          aria-label={t("journal_ask")}
          disabled={busy || (value.trim() === "" && attachment === null)}
          onClick={() => send(value)}
        >
          <SendIcon size={16} />
        </button>
      </div>

      {attachment !== null && (
        <div className="composer__attachment">
          <img
            className="composer__thumb"
            src={attachment}
            alt={t("journal_screenshot_attached")}
          />
          <button
            type="button"
            className="composer__remove"
            aria-label={t("journal_remove_screenshot")}
            title={t("journal_remove_screenshot")}
            onClick={onRemoveAttachment}
          >
            ×
          </button>
          <span className="composer__hint">
            {t("journal_screenshot_attached_text_optional")}
          </span>
        </div>
      )}

      {presets.length > 0 && (
        <div className="composer__presets">
          {presets.map((preset) => (
            <button
              key={preset}
              type="button"
              className="composer__preset"
              title={preset}
              disabled={busy}
              onClick={() => send(preset, "preset")}
            >
              {preset}
            </button>
          ))}
        </div>
      )}

      {error && <p className="composer__error">{error}</p>}
    </div>
  );
}
