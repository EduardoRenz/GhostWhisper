import { useCallback, useEffect, useState } from "react";
import { invokeSafe } from "../../adapters/ipc/client";
import { captureClear, capturePending } from "../../adapters/ipc/screen";
import { subscribe } from "../../adapters/ipc/events";
import { CloseIcon } from "../../components/icons";

/**
 * Prévia do print capturado (janela `capture`, BR-055).
 *
 * Fica no **canto superior direito** e mostra a captura com um botão de
 * remover — como no Perssua. O print **não é enviado** ao capturar: ele fica
 * anexado até o usuário confirmar a pergunta na barra de pesquisa.
 */
export function CapturePreviewWindow() {
  const [preview, setPreview] = useState<string | null>(null);

  const load = useCallback(async () => {
    const pending = await capturePending();
    if (!pending) {
      setPreview(null);
      return;
    }
    setPreview(`data:${pending.mime};base64,${pending.data}`);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // O core avisa quando um print novo chega/sai.
  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    void subscribe<null>("capture_pending", () => {
      void load();
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        stop = unlisten;
      }
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [load]);

  // Esc remove o print pendente.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void captureClear();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (!preview) {
    return null;
  }

  return (
    <div className="capture-preview" data-interactive>
      <div className="capture-preview__card">
        <img className="capture-preview__image" src={preview} alt="Print capturado" />
        <button
          type="button"
          className="capture-preview__remove"
          title="Remover o print"
          aria-label="Remover o print"
          onClick={() => void captureClear()}
        >
          <CloseIcon size={14} />
        </button>
      </div>
      <p className="capture-preview__hint">Anexado à pergunta</p>
    </div>
  );
}

/** Reexporta o invoke para o teste (fora do runtime vira no-op). */
export { invokeSafe };
