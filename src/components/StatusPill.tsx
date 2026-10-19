interface StatusPillProps {
  label: string;
  /** Tom: IA (violeta), foco (cyan) ou sucesso (esmeralda). */
  tone?: "ai" | "accent" | "success";
  /** Dica de ferramenta (opcional). */
  title?: string;
  className?: string;
}

/**
 * Pill de status com spinner (role="status").
 *
 * Usado **dentro do bloco de conteúdo** que está sendo gerado — resumo e
 * sugestão — para o usuário saber que é *aquele* texto que está sendo
 * criado/atualizado, e não outro. Reutilizável entre as janelas (BR-016).
 */
export function StatusPill({
  label,
  tone = "ai",
  title,
  className,
}: StatusPillProps) {
  return (
    <div
      className={[
        "status-pill",
        `status-pill--${tone}`,
        className ?? "",
      ]
        .filter(Boolean)
        .join(" ")}
      role="status"
      aria-live="polite"
      title={title}
    >
      <span className="status-pill__spinner" aria-hidden="true" />
      <span>{label}</span>
    </div>
  );
}
