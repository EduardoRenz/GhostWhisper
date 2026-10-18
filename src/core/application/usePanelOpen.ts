import { useEffect, useState } from "react";
import { panelIsOpen, type PanelLabel } from "../../adapters/ipc/panels";
import { subscribe } from "../../adapters/ipc/events";

/**
 * `true` enquanto a janela do painel está visível.
 *
 * O core emite `panel_opened`/`panel_hidden`; no primeiro mount consultamos
 * `panel_is_open` para não depender de um evento que já passou.
 */
export function usePanelOpen(label: PanelLabel): boolean {
  const [opened, setOpened] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const unsubscribers: Array<() => void> = [];

    void panelIsOpen(label).then((open) => {
      if (!cancelled && open) {
        setOpened(true);
      }
    });

    void subscribe<string>("panel_opened", (payload) => {
      if (payload === label) {
        setOpened(true);
      }
    }).then((stop) => {
      if (cancelled) {
        stop();
      } else {
        unsubscribers.push(stop);
      }
    });

    void subscribe<string>("panel_hidden", (payload) => {
      if (payload === label) {
        setOpened(false);
      }
    }).then((stop) => {
      if (cancelled) {
        stop();
      } else {
        unsubscribers.push(stop);
      }
    });

    return () => {
      cancelled = true;
      unsubscribers.forEach((stop) => stop());
    };
  }, [label]);

  return opened;
}
