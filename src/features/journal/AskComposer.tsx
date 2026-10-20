import {
  CameraIcon,
  CloseIcon,
  SearchIcon,
  SendIcon,
} from "../../components/icons";

interface AskComposerProps {
  /** Texto digitado. */
  value: string;
  onChange: (value: string) => void;
  /** Perguntas prontas (chips). */
  savedQuestions: string[];
  /** Envia a pergunta (a origem distingue pergunta pronta de digitada). */
  onSubmit: (question: string, source: "search" | "preset") => void;
  /** Captura a tela e anexa à pergunta. */
  onCapture: () => void;
  /** Miniatura da imagem anexada (data URL). */
  attachment: string | null;
  /** Remove a imagem anexada. */
  onRemoveAttachment: () => void;
  /** Cola uma imagem da área de transferência (data URL). */
  onPasteImage: (dataUrl: string) => void;
  /** Salva a pergunta atual como pronta. */
  onSavePreset: () => void;
  /** Abre o editor das perguntas prontas. */
  onOpenPresets: () => void;
  /** Uma pergunta está em curso. */
  busy: boolean;
  /** Mensagem de erro (ex.: print indisponível). */
  error: string | null;
}

/**
 * Composer da **Busca** (aba "Sessão", BR-053).
 *
 * Campo de pergunta + print de tela + chips de perguntas prontas. `Enter`
 * envia; `Shift+Enter` quebra a linha.
 */
export function AskComposer({
  value,
  onChange,
  savedQuestions,
  onSubmit,
  onCapture,
  attachment,
  onRemoveAttachment,
  onPasteImage,
  onSavePreset,
  onOpenPresets,
  busy,
  error,
}: AskComposerProps) {
  const submit = () => {
    const question = value.trim();
    // Sem texto só é permitido **com print anexado** (a IA decide).
    if ((question === "" && !attachment) || busy) {
      return;
    }
    onSubmit(question, "search");
  };

  return (
    <div className="ask" data-interactive>
      <div className="ask__field">
        <textarea
          className="ask__input"
          value={value}
          rows={2}
          placeholder="Pergunte à IA sobre a reunião…"
          aria-label="Pergunta para a IA"
          disabled={busy}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              submit();
            }
          }}
          onPaste={(event) => {
            // Colar imagem: o caminho que funciona também em Wayland (BR-055).
            const item = Array.from(event.clipboardData.items).find((entry) =>
              entry.type.startsWith("image/"),
            );
            if (!item) {
              return;
            }
            const file = item.getAsFile();
            if (!file) {
              return;
            }
            event.preventDefault();
            const reader = new FileReader();
            reader.onload = () => {
              if (typeof reader.result === "string") {
                onPasteImage(reader.result);
              }
            };
            reader.readAsDataURL(file);
          }}
        />
        <div className="ask__actions">
          <button
            type="button"
            className="ask__icon"
            title="Capturar a tela e anexar à pergunta"
            aria-label="Capturar a tela"
            disabled={busy || attachment !== null}
            onClick={onCapture}
          >
            <CameraIcon size={16} />
          </button>
          <button
            type="button"
            className="ask__icon"
            title="Salvar esta pergunta como pronta"
            aria-label="Salvar pergunta pronta"
            disabled={busy || value.trim() === ""}
            onClick={onSavePreset}
          >
            <SearchIcon size={16} />
          </button>
          <button
            type="button"
            className="ask__icon"
            title="Editar as perguntas prontas"
            aria-label="Editar perguntas prontas"
            disabled={busy}
            onClick={onOpenPresets}
          >
            <span aria-hidden="true">⋯</span>
          </button>
          <button
            type="button"
            className="ask__send"
            title="Perguntar"
            aria-label="Perguntar"
            disabled={busy || (value.trim() === "" && !attachment)}
            onClick={submit}
          >
            <SendIcon size={16} />
          </button>
        </div>
      </div>

      {attachment && (
        <div className="ask__attachment">
          <img src={attachment} alt="Print anexado" className="ask__thumb" />
          <button
            type="button"
            className="ask__remove"
            title="Remover o print"
            aria-label="Remover o print"
            onClick={onRemoveAttachment}
          >
            <CloseIcon size={14} />
          </button>
        </div>
      )}

      {savedQuestions.length > 0 && (
        <div className="ask__presets" role="list" aria-label="Perguntas prontas">
          {savedQuestions.map((question) => (
            <button
              key={question}
              type="button"
              className="ask__preset"
              role="listitem"
              title={`Perguntar: ${question}`}
              disabled={busy}
              onClick={() => onSubmit(question, "preset")}
            >
              {question}
            </button>
          ))}
        </div>
      )}

      {error && <p className="ask__error">{error}</p>}
    </div>
  );
}
