/**
 * Idioma da interface (l10n).
 *
 * O valor persistido nas settings é a fonte da verdade; o frontend nunca
 * detecta o idioma por conta própria depois da primeira inicialização — o core
 * resolve o padrão do sistema uma única vez no boot (`ui_language`).
 */

/** Idiomas de interface suportados hoje. */
export type UiLanguage = "pt" | "en";

/** Idiomas na ordem de exibição. */
export const UI_LANGUAGES: UiLanguage[] = ["pt", "en"];

/** Idioma usado até as settings carregarem (o produto nasceu em pt-BR). */
export const DEFAULT_UI_LANGUAGE: UiLanguage = "pt";

/** Nome do idioma no próprio idioma (não traduzir). */
export const UI_LANGUAGE_NAMES: Record<UiLanguage, string> = {
  pt: "Português (Brasil)",
  en: "English (US)",
};

/** Normaliza um valor arbitrário para um idioma suportado. */
export function normalizeUiLanguage(
  value: string | null | undefined,
): UiLanguage | null {
  const normalized = value?.trim().toLowerCase();
  if (normalized === "pt" || normalized === "en") {
    return normalized;
  }
  return null;
}

/**
 * Converte um locale do sistema (`pt_BR.UTF-8`, `en-US`, `es_ES`) no idioma da
 * interface. Português → pt; qualquer outro idioma conhecido → inglês; sem
 * informação → inglês (fallback neutro do l10n).
 *
 * Espelha `domain::settings::ui_language_from_locale` no core.
 */
export function uiLanguageFromLocale(locale: string | null | undefined): UiLanguage {
  const language = locale?.trim().toLowerCase().split(/[._@-]/)[0];
  return language === "pt" ? "pt" : "en";
}

/** Locale BCP-47 usado por `Intl`/`toLocaleString`. */
export function intlLocale(language: UiLanguage): string {
  return language === "en" ? "en-US" : "pt-BR";
}
