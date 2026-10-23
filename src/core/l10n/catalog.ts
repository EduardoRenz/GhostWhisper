/**
 * Catálogo e motor de tradução do frontend.
 *
 * As chaves são nomes **semânticos em snake_case** (`settings_save_changes`,
 * `common_close`); os textos vivem em [`locales/pt.ts`](locales/pt.ts) e
 * [`locales/en.ts`](locales/en.ts). O português é o idioma de origem e o
 * fallback do catálogo.
 *
 * Regra (`.agents/rules.md` §5.1): toda cópia visível ao usuário no frontend
 * passa por `t("chave_snake_case")`; texto solto em JSX/atributos é proibido.
 */

import { PT, type TranslationKey } from "./locales/pt";
import { EN } from "./locales/en";
import { DEFAULT_UI_LANGUAGE, type UiLanguage } from "./language";
import { getUiLanguage } from "./store";

export type { TranslationKey };

/** Parâmetros de interpolação (`{nome}`). */
export type TranslationParams = Record<string, string | number>;

/** Catálogos por idioma. */
const CATALOGS: Record<UiLanguage, Partial<Record<TranslationKey, string>>> = {
  pt: PT,
  en: EN,
};

/** Interpola `{nome}` com os parâmetros fornecidos. */
export function interpolate(
  template: string,
  params?: TranslationParams,
): string {
  if (!params) {
    return template;
  }
  return template.replace(/\{(\w+)\}/g, (match, name: string) => {
    const value = params[name];
    return value === undefined ? match : String(value);
  });
}

/**
 * Traduz uma chave para o idioma ativo.
 *
 * Fallback: idioma ativo → português (idioma de origem) → a própria chave. O
 * teste de catálogo garante que as duas línguas têm o mesmo conjunto de chaves.
 */
export function translate(
  key: TranslationKey,
  language: UiLanguage = getUiLanguage(),
  params?: TranslationParams,
): string {
  const template =
    CATALOGS[language]?.[key] ?? PT[key] ?? (key as string);
  return interpolate(template, params);
}

/** `true` quando a chave existe no catálogo. */
export function hasTranslation(key: string): key is TranslationKey {
  return key in PT;
}

/** Versão do catálogo usada nos testes de completude. */
export const TRANSLATION_KEYS = Object.keys(PT) as TranslationKey[];

export { EN, PT };
export type { UiLanguage };
export { DEFAULT_UI_LANGUAGE };
