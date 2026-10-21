import { useEffect, useRef, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import {
  getSettings,
  HUD_OPACITY_PREVIEW_EVENT,
} from "../../adapters/ipc/settings";
import {
  DEFAULT_OVERLAY_OPACITY,
  clampOverlayOpacity,
} from "../domain/settings";

/** Aplica a opacidade global no documento atual (todas as janelas). */
export function applyHudOpacity(value: number): void {
  if (typeof document === "undefined") {
    return;
  }
  document.documentElement.style.setProperty(
    "--hud-alpha",
    String(clampOverlayOpacity(value)),
  );
}

/**
 * Mantém `--hud-alpha` sincronizado com a opacidade global (BR-062).
 *
 * A opacidade vale para a superfície de **todas** as janelas: o root do
 * documento recebe o mesmo valor que o notch usava. O slider de Configurações
 * publica `hud_opacity_preview` antes de salvar, então o preview é aplicado em
 * todas as janelas abertas sem esperar o autosave.
 */
export function useHudOpacity(): number {
  const [opacity, setOpacity] = useState(DEFAULT_OVERLAY_OPACITY);
  // Evita que um `settings_changed` disparado por um save com valor antigo
  // (ou pelo próprio debounce) pisoteie um preview que acabou de chegar.
  const previewAt = useRef(0);

  useEffect(() => {
    let cancelled = false;
    let stopPreview: (() => void) | undefined;
    let stopSettings: (() => void) | undefined;

    const reload = async () => {
      const loaded = await getSettings();
      if (cancelled || !loaded) {
        return;
      }
      if (Date.now() - previewAt.current < 800) {
        return;
      }
      setOpacity(clampOverlayOpacity(loaded.overlay_opacity));
    };

    void reload();
    void subscribe<number>(HUD_OPACITY_PREVIEW_EVENT, (value) => {
      if (typeof value !== "number" || !Number.isFinite(value)) {
        return;
      }
      previewAt.current = Date.now();
      setOpacity(clampOverlayOpacity(value));
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
    applyHudOpacity(opacity);
  }, [opacity]);

  return opacity;
}
