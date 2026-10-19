import { acceleratorBadges } from "../core/domain/shortcuts";

interface ShortcutBadgesProps {
  /** *Accelerator* (ex.: `"CommandOrControl+Shift+S"`). */
  accelerator: string;
  className?: string;
}

/**
 * Desenha o atalho em *badges* (`Ctrl` `Shift` `S`) ao lado de uma ação.
 *
 * É decorativo para leitores de tela (`aria-hidden`): o nome acessível do botão
 * já descreve a ação; o atalho costuma acompanhar em `title`. Atalho vazio não
 * renderiza nada.
 */
export function ShortcutBadges({ accelerator, className }: ShortcutBadgesProps) {
  const badges = acceleratorBadges(accelerator);
  if (badges.length === 0) {
    return null;
  }
  return (
    <span
      className={["shortcut-badges", className ?? ""].filter(Boolean).join(" ")}
      aria-hidden="true"
    >
      {badges.map((badge, index) => (
        <kbd key={`${badge}-${index}`} className="shortcut-badge">
          {badge}
        </kbd>
      ))}
    </span>
  );
}
