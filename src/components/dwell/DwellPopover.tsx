import type { CSSProperties } from "react";
import { CheckIcon, TouchIcon } from "../icons";
import { dwellLabel } from "../../core/domain/dwell";

interface DwellPopoverProps {
  dwellMs: number;
  /** O "Sim" está com o tempo correndo. */
  arming: boolean;
  /** Já confirmou e está fechando. */
  busy: boolean;
  onArmStart: () => void;
  onArmStop: () => void;
  onConfirm: () => void;
}

/**
 * Conteúdo da janela `dwell` (BR-036): dica "Sem clique" + **Sim**.
 *
 * O botão "Sim" usa o mesmo fill do gatilho; o segundo estágio é controlado pela
 * janela (`DwellPopoverWindow`).
 */
export function DwellPopover({
  dwellMs,
  arming,
  busy,
  onArmStart,
  onArmStop,
  onConfirm,
}: DwellPopoverProps) {
  return (
    <div
      className="dwell-popover"
      role="dialog"
      aria-label="Confirmar ação"
      style={{ "--dwell-ms": `${dwellMs}ms` } as CSSProperties}
    >
      <span className="dwell-popover__hint">
        <TouchIcon size={14} />
        Sem clique
      </span>
      <button
        type="button"
        className="dwell-target dwell-confirm"
        data-arming={arming}
        title="Permanecer para confirmar (clique também confirma)"
        disabled={busy}
        onPointerEnter={onArmStart}
        onPointerLeave={onArmStop}
        onPointerCancel={onArmStop}
        onClick={onConfirm}
      >
        <span className="dwell-fill" aria-hidden="true" />
        <span className="dwell-line" aria-hidden="true" />
        <CheckIcon size={14} />
        <span>Sim</span>
        <span className="dwell-badge">{dwellLabel(dwellMs)}</span>
      </button>
    </div>
  );
}
