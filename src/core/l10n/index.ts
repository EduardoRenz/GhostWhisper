/**
 * API pública do l10n do frontend.
 *
 * Uso típico:
 *
 * ```tsx
 * const { t } = useTranslation();
 * <button title={t("index_close")}>{t("index_close")}</button>
 * ```
 *
 * O idioma vem do store global, alimentado pelas settings (`ui_language`). O
 * idioma de fallback é o português; o inglês é injetado pelo catálogo.
 */

import { useCallback, useSyncExternalStore } from "react";
import { intlLocale, type UiLanguage } from "./language";
import {
  translate,
  type TranslationKey,
  type TranslationParams,
} from "./catalog";
import { getUiLanguage, subscribeUiLanguage } from "./store";

export {
  DEFAULT_UI_LANGUAGE,
  UI_LANGUAGES,
  UI_LANGUAGE_NAMES,
  intlLocale,
  normalizeUiLanguage,
  uiLanguageFromLocale,
  type UiLanguage,
} from "./language";
export { getUiLanguage, setUiLanguage, subscribeUiLanguage } from "./store";
export {
  EN,
  PT,
  TRANSLATION_KEYS,
  hasTranslation,
  interpolate,
  translate,
  type TranslationKey,
  type TranslationParams,
} from "./catalog";

export interface Translation {
  /** Idioma ativo. */
  language: UiLanguage;
  /** Locale BCP-47 para `Intl`/`toLocaleString`. */
  locale: string;
  /** Traduz uma chave para o idioma ativo. */
  t: (key: TranslationKey, params?: TranslationParams) => string;
}

/** Hook de tradução (re-renderiza quando o idioma muda). */
export function useTranslation(): Translation {
  const language = useSyncExternalStore(
    subscribeUiLanguage,
    getUiLanguage,
    getUiLanguage,
  );
  const t = useCallback(
    (key: TranslationKey, params?: TranslationParams) =>
      translate(key, language, params),
    [language],
  );
  return { language, locale: intlLocale(language), t };
}
