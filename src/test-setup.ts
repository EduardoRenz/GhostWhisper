/**
 * Setup comum dos testes do frontend.
 *
 * O idioma é estado global do módulo de l10n; cada teste começa em português
 * (idioma de origem) para não herdar uma troca feita pelo teste anterior.
 */

import { beforeEach } from "vitest";
import { setUiLanguage } from "./core/l10n";

beforeEach(() => {
  setUiLanguage("pt");
});
