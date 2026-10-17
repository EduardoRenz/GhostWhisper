import { useEffect } from "react";
import { invokeSafe } from "../../adapters/ipc/client";

/** Retângulo em CSS px relativo à viewport. */
export interface InteractiveRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Coleta os elementos `[data-interactive]` e informa suas regiões ao core,
 * que as usa para decidir o click-through do overlay.
 */
export function collectInteractiveRegions(
  root: ParentNode = document,
): InteractiveRegion[] {
  const nodes = root.querySelectorAll<HTMLElement>("[data-interactive]");
  return Array.from(nodes).map((el) => {
    const rect = el.getBoundingClientRect();
    return {
      x: Math.round(rect.left),
      y: Math.round(rect.top),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    };
  });
}

/**
 * Publica as regiões interativas ao core e as mantém atualizadas.
 *
 * Republisha ao montar, em resize e sempre que o DOM muda (troca de superfície,
 * início de sessão, etc.), com debounce via `requestAnimationFrame`.
 */
export function useInteractiveRegions(): void {
  useEffect(() => {
    let frame = 0;
    const publish = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        void invokeSafe("overlay_set_interactive_regions", {
          regions: collectInteractiveRegions(),
        });
      });
    };

    publish();
    window.addEventListener("resize", publish);

    const observer = new MutationObserver(publish);
    observer.observe(document.body, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["data-interactive", "class", "style"],
    });

    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", publish);
      observer.disconnect();
    };
  }, []);
}
