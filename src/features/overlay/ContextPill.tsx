import { useTranslation } from "../../core/l10n";
import type { ScreenContextEvent, ScreenContextState } from "../../core/domain/screenContext";

interface ContextPillProps {
  /** Estado derivado (BR-106): hidden não renderiza nada. */
  state: ScreenContextState;
  /** Último evento do core (estado ativo). */
  event: ScreenContextEvent | null;
  /** Checkbox marcado (contexto é lido e vai na pergunta) — BR-106. */
  checked?: boolean;
  /**
   * Marca/desmarca a preferência. Presente, a **pill inteira** vira checkbox
   * em todos os estados visíveis: **um** clique alterna (liga/desliga) e a
   * opção nunca some.
   */
  onToggle?: () => void;
}

/**
 * Pill de contexto da AskBar e da aba Sessão (BR-106, PRD §3.3).
 *
 * Mostra **qual contexto o copiloto está enxergando**: `🌐 app · título` com
 * a prévia no tooltip — o usuário inspeciona o que será enviado antes de
 * perguntar. Com `onToggle`, a pill inteira é um **checkbox** (role/
 * aria-checked): cada clique alterna incluir/excluir, e clicar em qualquer
 * parte dela (caixa, ícone ou texto) conta como **um** clique.
 *
 * Desmarcado, o estado é `paused`: nada é lido nem enviado (zero
 * processamento), e a pill permanece clicável para religar.
 */
export function ContextPill({
  state,
  event,
  checked = true,
  onToggle,
}: ContextPillProps) {
  const { t } = useTranslation();

  if (state === "hidden") {
    return null;
  }

  const active = state === "active" && event;
  const paused = state === "paused";

  const title = paused
    ? t("ask_bar_context_paused_title")
    : active
      ? t("ask_bar_context_preview", { preview: event.preview })
      : state === "loading"
        ? t("ask_bar_context_loading_aria")
        : state === "empty"
          ? t("ask_bar_context_empty_title")
          : t("ask_bar_context_unavailable_aria");

  const label = paused
    ? t("ask_bar_context_paused")
    : active
      ? t("ask_bar_context_active", {
          app: event.app,
          title: event.window_title,
        })
      : state === "loading"
        ? t("ask_bar_context_loading")
        : state === "empty"
          ? t("ask_bar_context_empty")
          : t("ask_bar_context_unavailable");

  const stateAria = paused
    ? t("ask_bar_context_paused_aria")
    : active
      ? t("ask_bar_context_active_aria", {
          app: event.app,
          title: event.window_title,
        })
      : state === "loading"
        ? t("ask_bar_context_loading_aria")
        : state === "empty"
          ? t("ask_bar_context_empty_aria")
          : t("ask_bar_context_unavailable_aria");

  const icon = (
    <span className="askbar__context-icon" aria-hidden="true">
      {paused ? "⏸" : active ? "🌐" : state === "loading" ? "◌" : "○"}
    </span>
  );
  const labelNode = <span className="askbar__context-label">{label}</span>;

  // Checkbox (BR-106): a pill inteira alterna — um clique por alternância,
  // em todos os estados visíveis (inclusive pausado/sem contexto).
  if (onToggle) {
    const action = paused
      ? t("ask_bar_context_toggle_hint_paused")
      : active
        ? checked
          ? t("ask_bar_context_toggle_included", {
              app: event.app,
              title: event.window_title,
            })
          : t("ask_bar_context_toggle_excluded", {
              app: event.app,
              title: event.window_title,
            })
        : checked
          ? t("ask_bar_context_toggle_hint_included")
          : t("ask_bar_context_toggle_hint_excluded");

    return (
      <button
        type="button"
        role="checkbox"
        aria-checked={checked}
        aria-live="polite"
        aria-label={action}
        title={title}
        className={`askbar__context askbar__context--${state} askbar__context--toggle${
          checked ? "" : " askbar__context--unchecked"
        }`}
        data-testid="context-pill"
        onClick={onToggle}
      >
        <span className="askbar__context-check" aria-hidden="true">
          {checked ? "✓" : ""}
        </span>
        {icon}
        {labelNode}
      </button>
    );
  }

  return (
    <span
      className={`askbar__context askbar__context--${state}`}
      role="status"
      aria-live="polite"
      aria-label={stateAria}
      title={title}
      data-testid="context-pill"
    >
      {icon}
      {labelNode}
    </span>
  );
}
