import { useEffect, useRef } from "react";
import { DragHandle } from "../../components/DragHandle";
import { ShortcutBadges } from "../../components/ShortcutBadges";
import { CameraIcon, SearchIcon } from "../../components/icons";

interface AskBarProps {
  /** Texto da pergunta. */
  value: string;
  onChange: (value: string) => void;
  /** Envia a pergunta (Enter). */
  onSubmit: (question: string) => void;
  /** Captura o monitor sob o cursor e pergunta (com ou sem texto). */
  onCapture: () => void;
  /** Foca o campo (atalho "Perguntar"). */
  focusSignal: number;
  /** Uma captura está em curso. */
  busy: boolean;
  /** Já existe um print anexado (a captura fica desabilitada). */
  attached?: boolean;
  /** Inicia o arraste da janela (alça à esquerda, como no notch). */
  onStartDrag?: () => void;
  /** Atalho que abre a barra (padrão `CommandOrControl+K`). */
  askAccelerator?: string;
  /** Atalho da captura de tela (padrão `Alt+C`). */
  captureAccelerator?: string;
  /** Perguntas prontas (chips abaixo da barra). */
  presets?: string[];
  /** Usa uma pergunta pronta (envia direto). */
  onPreset?: (text: string) => void;
}

/**
 * Barra "Perguntar" — a superfície **padrão** e discreta do overlay (BR-052).
 *
 * Layout de referência (Perssua): lupa + campo + atalho │ botão **Capturar
 * Tela**. Em repouso, as **perguntas prontas** aparecem como chips abaixo da
 * barra (BR-054). A sessão só abre depois da pergunta/captura (o painel é
 * aberto por quem consome os callbacks).
 */
export function AskBar({
  value,
  onChange,
  onSubmit,
  onCapture,
  focusSignal,
  busy,
  attached = false,
  onStartDrag,
  askAccelerator = "CommandOrControl+K",
  captureAccelerator = "Alt+C",
  presets = [],
  onPreset,
}: AskBarProps) {
  const input = useRef<HTMLInputElement>(null);

  // O atalho global pede o foco (evento `focus_ask`).
  useEffect(() => {
    if (focusSignal > 0) {
      input.current?.focus();
    }
  }, [focusSignal]);

  // As perguntas prontas ficam à vista **em repouso**; ao digitar, somem para
  // não disputar espaço com o que está sendo escrito.
  const showPresets =
    presets.length > 0 && value.trim() === "" && !busy && Boolean(onPreset);

  return (
    <>
      <div className="askbar" data-interactive>
        {onStartDrag && (
          <DragHandle
            className="askbar__drag"
            label="Mover a barra de pesquisa"
            onStartDrag={onStartDrag}
          />
        )}

        <span className="askbar__icon" aria-hidden="true">
          <SearchIcon size={18} />
        </span>

        <input
          ref={input}
          className="askbar__input"
          type="text"
          value={value}
          placeholder="Perguntar ao GhostWhisper sobre a tela ou áudio…"
          aria-label="Perguntar ao GhostWhisper"
          disabled={busy}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              onSubmit(value.trim());
            }
          }}
        />

        <span className="askbar__keys">
          <ShortcutBadges accelerator={askAccelerator} />
        </span>

        <span className="askbar__divider" aria-hidden="true" />

        <button
          type="button"
          className={`askbar__capture${attached ? " askbar__capture--attached" : ""}`}
          title={
            attached
              ? "Já há um print anexado — confirme a pergunta para enviá-lo"
              : "Captura o monitor sob o cursor e anexa à pergunta (Alt+C)"
          }
          aria-label="Capturar Tela"
          disabled={busy || attached}
          onClick={onCapture}
        >
          <span className="askbar__capture-icon" aria-hidden="true">
            <CameraIcon size={16} />
          </span>
          <span className="askbar__capture-label">
            {attached ? "Print anexado" : "Capturar Tela"}
          </span>
          <span className="askbar__keys askbar__keys--subtle">
            <ShortcutBadges accelerator={captureAccelerator} />
          </span>
        </button>
      </div>

      {showPresets && (
        <div className="askbar__presets" data-interactive>
          {presets.map((preset) => (
            <button
              key={preset}
              type="button"
              className="askbar__preset"
              title={preset}
              onClick={() => onPreset?.(preset)}
            >
              {preset}
            </button>
          ))}
        </div>
      )}
    </>
  );
}
