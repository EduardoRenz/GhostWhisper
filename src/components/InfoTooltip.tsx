/**
 * Tooltip de ajuda dos campos de agente (BR-081).
 *
 * O balão é renderizado em **portal** (`document.body`, `position: fixed`) e
 * posicionado por `tooltipPosition`: escolhe o lado com espaço e prende nas
 * bordas da janela. Assim ele não é cortado por containers com `overflow`
 * (o corpo rolável das Configurações, o modal) e se adapta ao tamanho da
 * janela — inclusive no painel da direita dos cards.
 *
 * Abre no hover **e** no foco (teclado); `Esc` fecha. O texto fica ligado ao
 * gatilho por `aria-describedby`.
 */

import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { tooltipPosition } from "../core/domain/tooltip";
import { InfoIcon } from "./icons";

export interface InfoTooltipProps {
  /** Nome do campo (compõe o rótulo acessível do botão). */
  label: string;
  /** Texto explicativo. */
  text: string;
  className?: string;
}

interface Position {
  top: number;
  left: number;
}

/** Ícone "i" com um balão de ajuda inteligente. */
export function InfoTooltip({ label, text, className }: InfoTooltipProps) {
  const id = useId();
  const anchorRef = useRef<HTMLSpanElement>(null);
  const bubbleRef = useRef<HTMLSpanElement>(null);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [position, setPosition] = useState<Position | null>(null);

  const open = hovered || focused;

  /** Mede o gatilho e o balão e calcula a posição na viewport. */
  const place = useCallback(() => {
    const anchor = anchorRef.current;
    const bubble = bubbleRef.current;
    if (!anchor || !bubble) {
      return;
    }
    const rect = anchor.getBoundingClientRect();
    const size = bubble.getBoundingClientRect();
    const next = tooltipPosition(
      {
        top: rect.top,
        left: rect.left,
        width: rect.width,
        height: rect.height,
      },
      { width: size.width, height: size.height },
      { width: window.innerWidth, height: window.innerHeight },
    );
    setPosition({ top: next.top, left: next.left });
  }, []);

  // Antes do paint: o balão já nasce na posição certa (sem "pulo").
  useLayoutEffect(() => {
    if (open) {
      place();
    }
  }, [open, place]);

  // Recalcula ao rolar/redimensionar; o balão acompanha o gatilho.
  useEffect(() => {
    if (!open) {
      return;
    }
    let frame = 0;
    const update = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(place);
    };
    window.addEventListener("scroll", update, true);
    window.addEventListener("resize", update);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("scroll", update, true);
      window.removeEventListener("resize", update);
    };
  }, [open, place]);

  const bubble =
    typeof document === "undefined" ? null : (
      createPortal(
        <span
          ref={bubbleRef}
          id={id}
          role="tooltip"
          className="info-tooltip__bubble"
          data-open={open && position ? "true" : "false"}
          style={position ? { top: position.top, left: position.left } : undefined}
        >
          {text}
        </span>,
        document.body,
      )
    );

  return (
    <span
      ref={anchorRef}
      className={["info-tooltip", className].filter(Boolean).join(" ")}
    >
      <button
        type="button"
        className="info-tooltip__button"
        aria-label={`Ajuda: ${label}`}
        aria-describedby={id}
        onPointerEnter={() => setHovered(true)}
        onPointerLeave={() => setHovered(false)}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            setHovered(false);
            setFocused(false);
            event.currentTarget.blur();
          }
        }}
      >
        <InfoIcon size={12} />
      </button>
      {bubble}
    </span>
  );
}
