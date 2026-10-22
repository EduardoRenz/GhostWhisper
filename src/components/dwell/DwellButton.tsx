import { useRef, type CSSProperties, type ReactNode } from "react";
import { useDwellTimer } from "../../core/application/useDwellTimer";
import { useDwell } from "./DwellProvider";

interface DwellButtonProps {
  /** Ação executada no clique e ao completar a permanência. */
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
 * - **Hover** preenche por `dwell_ms`; ao completar, executa `onActivate`
 *   diretamente, sem confirmação intermediária.
 * - O preenchimento é uma animação CSS (`data-arming`), sem state por quadro.
 */
export function DwellButton({
  onActivate,
  className,
  pressed,
  title,
  disabled = false,
  children,
}: DwellButtonProps) {
  const dwell = useDwell();
  const fired = useRef(false);
  const timer = useDwellTimer(
    dwell.durationMs,
    () => {
      fired.current = true;
      onActivate();
    },
    dwell.enabled && !disabled,
  );

  const leave = () => {
    fired.current = false;
    timer.stop();
  };

  return (
    <button
      type="button"
      className={["dwell-target", className ?? ""].filter(Boolean).join(" ")}
      data-arming={timer.active}
      style={
        {
          "--dwell-ms": `${dwell.durationMs}ms`,
        } as CSSProperties
      }
      aria-pressed={pressed}
      title={title}
      disabled={disabled}
      onPointerEnter={() => {
        if (!fired.current) {
          timer.start();
        }
      }}
      onPointerMove={() => {
        if (!fired.current) {
          timer.start();
        }
      }}
      onPointerLeave={leave}
      onPointerCancel={leave}
      onClick={() => {
        fired.current = true;
        timer.stop();
        onActivate();
      }}
    >
      <span className="dwell-fill" aria-hidden="true" />
      <span className="dwell-line" aria-hidden="true" />
      {children}
    </button>
  );
}
