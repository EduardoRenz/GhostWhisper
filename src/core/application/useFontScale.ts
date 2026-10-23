import { useEffect, useRef, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import {
  FONT_SCALES_PREVIEW_EVENT,
  getSettings,
  type FontScalePreview,
} from "../../adapters/ipc/settings";
import {
  DEFAULT_AI_FONT_SCALE,
  DEFAULT_FONT_SCALE,
  clampAiFontScale,
  clampFontScale,
} from "../domain/settings";

/**
 * Base do `font-size` do `<html>`.
 *
 * Capturada uma vez, **antes** de qualquer zoom, para não brigar com a
 * preferência de fonte do sistema e para o cálculo nunca se acumular
 * (1,4 → 1,6 não pode virar 2,24).
 */
let rootFontBase: number | null = null;

function baseFontSizePx(): number {
  if (rootFontBase === null) {
    let value = 16;
    if (typeof document !== "undefined") {
      const computed = window
        .getComputedStyle(document.documentElement)
        .fontSize;
      const parsed = Number.parseFloat(computed);
      if (Number.isFinite(parsed) && parsed > 0) {
        value = parsed;
      }
    }
    rootFontBase = value;
  }
  return rootFontBase;
}

/**
 * Aplica as escalas de fonte no documento atual (BR-087).
 *
 * - **Global:** zoom no `<html>` (`rem` acompanha fontes *e* espaçamento, então
 *   botões e textos crescem juntos e nada estoura). Em 100 % a propriedade é
 *   removida: o app volta exatamente ao tamanho anterior.
 * - **IA:** `--oh-ai-font-scale` multiplica o texto gerado por IA.
 * - **Escopo:** só janelas de conteúdo (`enabled`). Notch e barra "Perguntar"
 *   têm tamanho/densidade próprios e ficam intactos.
 */
export function applyFontScale(
  fontScale: number,
  aiFontScale: number,
  enabled: boolean,
): void {
  if (typeof document === "undefined") {
    return;
  }
  const root = document.documentElement;
  root.style.setProperty(
    "--oh-ai-font-scale",
    enabled ? String(clampAiFontScale(aiFontScale)) : String(DEFAULT_AI_FONT_SCALE),
  );

  const scale = clampFontScale(fontScale);
  if (!enabled || scale === DEFAULT_FONT_SCALE) {
    root.style.removeProperty("font-size");
    return;
  }
  root.style.setProperty(
    "font-size",
    `calc(${baseFontSizePx()}px * ${scale})`,
  );
}

/** Escalas vigentes no documento. */
export interface FontScales {
  /** Escala global da UI (0,75–1,6). */
  fontScale: number;
  /** Bônus do texto de IA (1,0–1,5). */
  aiFontScale: number;
}

/**
 * Mantém as escalas de fonte sincronizadas com as settings (BR-087).
 *
 * Espelha `useHudOpacity`: o slider de Configurações publica
 * `font_scales_preview` a cada movimento, então o preview vale em todas as
 * janelas de conteúdo sem esperar o autosave.
 */
export function useFontScale(enabled = true): FontScales {
  const [scales, setScales] = useState<FontScales>({
    fontScale: DEFAULT_FONT_SCALE,
    aiFontScale: DEFAULT_AI_FONT_SCALE,
  });
  // Evita que um `settings_changed` de um save antigo pisoteie um preview
  // que acabou de chegar (mesmo guarda da opacidade).
  const previewAt = useRef(0);

  useEffect(() => {
    let cancelled = false;
    let stopPreview: (() => void) | undefined;
    let stopSettings: (() => void) | undefined;

    const apply = (preview: FontScalePreview) => {
      setScales({
        fontScale: clampFontScale(preview.font_scale),
        aiFontScale: clampAiFontScale(preview.ai_font_scale),
      });
    };

    const reload = async () => {
      const loaded = await getSettings();
      if (cancelled || !loaded) {
        return;
      }
      if (Date.now() - previewAt.current < 800) {
        return;
      }
      apply({
        font_scale: loaded.font_scale,
        ai_font_scale: loaded.ai_font_scale,
      });
    };

    void reload();
    void subscribe<FontScalePreview>(FONT_SCALES_PREVIEW_EVENT, (payload) => {
      if (
        !payload ||
        typeof payload.font_scale !== "number" ||
        typeof payload.ai_font_scale !== "number"
      ) {
        return;
      }
      previewAt.current = Date.now();
      apply(payload);
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        stopPreview = unlisten;
      }
    });
    void subscribe<null>("settings_changed", () => {
      void reload();
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        stopSettings = unlisten;
      }
    });

    return () => {
      cancelled = true;
      stopPreview?.();
      stopSettings?.();
    };
  }, []);

  useEffect(() => {
    applyFontScale(scales.fontScale, scales.aiFontScale, enabled);
  }, [scales.fontScale, scales.aiFontScale, enabled]);

  return scales;
}
