import { useState } from "react";
import { CameraIcon, SendIcon } from "../../components/icons";
import { useTranslation } from "../../core/l10n";
import type { ScreenContextEvent, ScreenContextState } from "../../core/domain/screenContext";
import { ContextPill } from "../overlay/ContextPill";

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
  /** Estado do contexto de tela (BR-106); `hidden` não renderiza a pill. */
  contextState?: ScreenContextState;
  /** Último evento de contexto (estado ativo). */
  contextEvent?: ScreenContextEvent | null;
  /** Checkbox de contexto marcado (vai na pergunta) — BR-106. */
  contextChecked?: boolean;
  /** Marca/desmarca o contexto; presente, a pill vira checkbox. */
  onToggleContext?: () => void;
}

/**
 * Composer **compacto** da aba Sessão (BR-053).
 *
 * Uma linha: câmera + campo + **pill de contexto** + enviar; as perguntas
 * prontas vêm como chips abaixo. `Enter` envia. Com um print anexado o texto é
 * **opcional** — a IA decide o que fazer com a imagem. Capturar nunca envia:
 * só anexa. A pill (BR-106, com a flag mestre ligada) é o mesmo checkbox da
 * AskBar: marca/desmarca o contexto de tela na pergunta.
 */
export function AskComposer({
  presets,
  attachment,
  busy,
  error,
  onSend,
  onCapture,
  onRemoveAttachment,
  contextState = "hidden",
  contextEvent = null,
  contextChecked = true,
  onToggleContext,
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

        {/* Pill de contexto (BR-106): mesma fonte da AskBar — checkbox de
            incluir/excluir o contexto na pergunta. Oculta sem a flag. */}
        <ContextPill
          state={contextState}
          event={contextEvent}
          checked={contextChecked}
          onToggle={onToggleContext}
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
