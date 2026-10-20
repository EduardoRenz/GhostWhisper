import { useCallback, useEffect, useState } from "react";
import { askHide, askOpen } from "../../adapters/ipc/ask";
import { panelStartDrag } from "../../adapters/ipc/panels";
import { journalAsk } from "../../adapters/ipc/journal";
import { panelOpenTab } from "../../adapters/ipc/panels";
import {
  captureClear,
  capturePending,
  screenCapture,
} from "../../adapters/ipc/screen";
import { subscribe } from "../../adapters/ipc/events";
import { useSettings } from "../../core/application/useSettings";
import { defaultBindings } from "../../core/domain/shortcuts";
import { AskBar } from "../overlay/AskBar";

/**
 * Janela **Perguntar** (a barra de pesquisa, BR-052).
 *
 * Aberta pela lupa do notch (com ou sem transcrição) e pelo atalho global.
 * Ao perguntar/capturar: a barra **some** e o painel abre na aba **Sessão** —
 * a resposta chega em streaming. A sessão recebe o contexto da transcrição
 * quando existe; sem transcrição, a pergunta funciona igual.
 */
export function AskWindow() {
  const [question, setQuestion] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [focusSignal, setFocusSignal] = useState(0);
  const { settings } = useSettings();
  /** Há um print anexado (pendente de envio). */
  const [attached, setAttached] = useState(false);

  /** Fecha a barra e abre a sessão (o diário). */
  const finish = useCallback(() => {
    setQuestion("");
    setError(null);
    void panelOpenTab("chat", "session");
    void askHide();
  }, []);

  /** Anexo pendente → imagem para o pedido (se houver). */
  const attachedImage = useCallback(async () => {
    const pending = await capturePending();
    return pending ? [{ mime: pending.mime, data: pending.data }] : [];
  }, []);

  /**
   * Envia a pergunta. O print anexado vai **junto**, mas só agora — capturar
   * não envia nada (BR-055).
   */
  const ask = useCallback(
    async (text: string) => {
      const trimmed = text.trim();
      try {
        const images = await attachedImage();
        // Com print, o texto é **opcional** (a IA decide o que fazer).
        if (trimmed === "" && images.length === 0) {
          return;
        }
        void journalAsk({
          question: trimmed,
          source: images.length > 0 ? "screenshot" : "search",
          images,
        }).catch((caught) => setError(String(caught)));
        if (images.length > 0) {
          await captureClear();
        }
        finish();
      } catch (caught) {
        setError(String(caught));
      }
    },
    [attachedImage, finish],
  );

  /** Captura e **anexa** (não envia): a prévia aparece no canto da tela. */
  const capture = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const shot = await screenCapture();
      if (!shot) {
        setError("Captura de tela indisponível — cole a imagem com Ctrl+V.");
        return;
      }
      setAttached(true);
      // A captura esconde/restaura as janelas: garante a barra de volta e
      // focada (o `show` logo após o `hide` pode ser ignorado pelo WM).
      await askOpen();
    } catch (caught) {
      setError(String(caught));
    } finally {
      setBusy(false);
    }
  }, []);

  // O anexo pode ser removido na prévia (×): acompanha o estado do core.
  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    void subscribe<null>("capture_changed", () => {
      void capturePending().then((pending) => {
        if (!cancelled) {
          setAttached(Boolean(pending));
        }
      });
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
  }, []);

  // Atalho global de captura e pedido de foco, vindos do core.
  useEffect(() => {
    let cancelled = false;
    const stops: Array<() => void> = [];
    void subscribe<null>("ask_focus", () => {
      setFocusSignal((previous) => previous + 1);
    }).then((stop) => {
      if (cancelled) {
        stop();
      } else {
        stops.push(stop);
      }
    });
    void subscribe<null>("capture_screen", () => {
      void capture();
    }).then((stop) => {
      if (cancelled) {
        stop();
      } else {
        stops.push(stop);
      }
    });
    return () => {
      cancelled = true;
      stops.forEach((stop) => stop());
    };
  }, [capture]);

  // Esc fecha a barra (sem perguntar).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void askHide();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const accelerator = (action: "focus_ask" | "capture_screen") =>
    settings.shortcuts.find((binding) => binding.action === action)
      ?.accelerator ??
    defaultBindings().find((binding) => binding.action === action)!
      .accelerator;

  return (
    <div className="ask-window" data-interactive>
      <AskBar
        value={question}
        onChange={setQuestion}
        onSubmit={(text) => void ask(text)}
        onCapture={() => void capture()}
        focusSignal={focusSignal}
        busy={busy}
        attached={attached}
        onStartDrag={() => void panelStartDrag()}
        askAccelerator={accelerator("focus_ask")}
        captureAccelerator={accelerator("capture_screen")}
      />
      {error && <p className="ask-window__error">{error}</p>}
    </div>
  );
}
