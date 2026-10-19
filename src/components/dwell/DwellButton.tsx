import { useRef, type CSSProperties, type ReactNode } from "react";
import { useDwellTimer } from "../../core/application/useDwellTimer";
import { useDwell } from "./DwellProvider";

interface DwellButtonProps {
  /** Identificador estável do gatilho (para manter o fill aberto). */
  triggerId: string;
  /** Ação executada no clique **e** na confirmação. */
  onActivate: () => void;
  /** Classe do visual existente (ex.: `notch-tabs__item`). */
  className?: string;
  /** `aria-pressed`, quando for alternância. */
  pressed?: boolean;
  title?: string;
  disabled?: boolean;
  children: ReactNode;
}

/**
 * Botão opt-in do drill-hover (BR-036).
 *
 * - **Clique** executa a ação na hora.
 * - **Hover** preenche por `dwell_ms` e abre a confirmação; o clique continua
 *   valendo como fallback.
 * - O preenchimento é uma animação CSS (`data-arming`), sem state por quadro.
 */
export function DwellButton({
  triggerId,
  onActivate,
  className,
  pressed,
  title,
  disabled = false,
  children,
}: DwellButtonProps) {
  const dwell = useDwell();
  const ref = useRef<HTMLButtonElement>(null);
  const timer = useDwellTimer(
    dwell.durationMs,
    () => {
      const element = ref.current;
      if (element) {
        dwell.request(triggerId, element.getBoundingClientRect(), onActivate);
      }
    },
    dwell.enabled && !disabled,
  );

  const open = dwell.openTrigger === triggerId;
  const arming = timer.active || open;

  return (
    <button
      ref={ref}
      type="button"
      className={["dwell-target", className ?? ""].filter(Boolean).join(" ")}
      data-arming={arming}
      style={
        {
          "--dwell-ms": `${dwell.durationMs}ms`,
        } as CSSProperties
      }
      aria-haspopup={dwell.enabled ? "dialog" : undefined}
      aria-pressed={pressed}
      title={title}
      disabled={disabled}
      onPointerEnter={() => {
        dwell.setTriggerHover(triggerId, true);
        if (!dwell.openTrigger) {
          timer.start();
        }
      }}
      onPointerLeave={() => {
        timer.stop();
        dwell.setTriggerHover(triggerId, false);
      }}
      onPointerCancel={() => {
        timer.stop();
        dwell.setTriggerHover(triggerId, false);
      }}
      onClick={() => {
        timer.stop();
        dwell.click(triggerId, onActivate);
      }}
    >
      <span className="dwell-fill" aria-hidden="true" />
      <span className="dwell-line" aria-hidden="true" />
      {children}
    </button>
  );
}
