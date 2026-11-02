/** Contexto de tela por acessibilidade — tipos do front (BR-105/BR-106). */

/**
 * Estado da pill de contexto na AskBar (PRD §3.3).
 *
 * - `hidden` — flag desligada (a pill não renderiza).
 * - `paused` — checkbox desmarcado: nenhuma leitura acontece; a pill mostra
 *   só a opção de religar (o contexto é limpo, não fica em cache).
 * - `loading` — AskBar abriu e o snapshot ainda não voltou.
 * - `active` — há contexto (app + título + prévia).
 * - `empty` — flag ligada, mas a janela não expõe contexto (terminal/canvas).
 * - `unavailable` — AT-SPI/acessibilidade inacessível.
 */
export type ScreenContextState =
  | "hidden"
  | "paused"
  | "loading"
  | "active"
  | "empty"
  | "unavailable";

/** Evento `screen_context` publicado pelo core (BR-106). */
export interface ScreenContextEvent {
  /** `false` → sem contexto (empty); `true` → ativo. */
  available: boolean;
  /**
   * A última leitura falhou (AT-SPI/D-Bus inacessível) — pill
   * "Contexto indisponível". Distingue de `available:false` com leitura ok
   * ("Sem contexto de tela").
   */
  failed?: boolean;
  /** Nome curto do app (`google-chrome`); vazio quando indisponível. */
  app: string;
  /** Título da janela (truncado); vazio quando indisponível. */
  window_title: string;
  /** Prévia do texto (≤ 300 chars); vazia quando indisponível. */
  preview: string;
  /** Momento do snapshot (ms, relógio de parede). */
  captured_at_ms: number;
}

/** Disponibilidade do recurso nas Configurações (linha de status). */
export type ScreenContextStatus = "disabled" | "available" | "unavailable";

/**
 * Deriva o estado da pill a partir da flag, do último resultado e do
 * checkbox.
 *
 * - `null` (sem consulta) → `loading` enquanto a consulta está em voo; a flag
 *   desligada é `hidden` **antes** — o chamador nunca deve chamar com flag
 *   desligada (a pill nem renderiza).
 * - Checkbox desmarcado → `paused` **antes** do evento: não há leitura, e um
 *   evento antigo não pode reaparecer (BR-106).
 * - `failed` vence `available`: mecanismo quebrou → "Contexto indisponível".
 */
export function screenContextState(
  enabled: boolean,
  event: ScreenContextEvent | null,
  checked = true,
): ScreenContextState {
  if (!enabled) return "hidden";
  if (!checked) return "paused";
  if (!event) return "loading";
  if (event.failed) return "unavailable";
  return event.available ? "active" : "empty";
}
