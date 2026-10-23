import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  closeMenu,
  menuState,
  resizeMenu,
  type MenuKind,
} from "../../adapters/ipc/overlay";
import { subscribe } from "../../adapters/ipc/events";
import { AgentMenu } from "./AgentMenu";
import { ChannelMenu } from "./ChannelMenu";

/**
 * Janela flutuante dos menus do notch (agentes/canais), BR-072/ISS-063.
 *
 * O notch não muda de tamanho nem de posição: o popup é uma janela própria,
 * mostrada acima/abaixo do gatilho pelo core. Fecha ao escolher, com `Escape`,
 * ao clicar na janela do notch ou quando perde o foco (clique fora).
 */
export function MenuWindow() {
  const [kind, setKind] = useState<MenuKind | null>(null);
  const [sequence, setSequence] = useState(0);
  const cardRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;

    void menuState()
      .then((loaded) => {
        if (!cancelled && (loaded === "agent" || loaded === "channel")) {
          setKind(loaded);
        }
      })
      .catch(() => undefined);

    void subscribe<string>("menu_show", (payload) => {
      if (payload === "agent" || payload === "channel") {
        setKind(payload);
        // Recarrega o catálogo a cada abertura (agentes podem ter mudado).
        setSequence((current) => current + 1);
      }
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        stop = unlisten;
      }
    });

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void closeMenu();
      }
    };
    document.addEventListener("keydown", onKeyDown);

    return () => {
      cancelled = true;
      stop?.();
      document.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  // Perda de foco = clique em outra janela/app → fecha o popup.
  useEffect(() => {
    const window_ = getCurrentWindow();
    let armed = false;
    let unlisten: (() => void) | undefined;
    void window_
      .onFocusChanged(({ payload }) => {
        if (payload) {
          armed = true;
        } else if (armed) {
          void closeMenu();
        }
      })
      .then((stop) => {
        unlisten = stop;
      });
    return () => unlisten?.();
  }, []);

  // Altura dinâmica: mede o conteúdo real da lista (scrollHeight) e pede ao
  // core para redimensionar/reposicionar. `+14` = borda do card (2) + padding
  // da janela (6 em cima, 6 embaixo).
  useLayoutEffect(() => {
    if (!kind) {
      return;
    }
    const card = cardRef.current;
    const list = card?.querySelector<HTMLElement>(".menu-list");
    if (!list) {
      return;
    }
    let frame = 0;
    let last = 0;
    const publish = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const height = Math.ceil(list.scrollHeight + 14);
        if (height > 0 && Math.abs(height - last) >= 1) {
          last = height;
          void resizeMenu(height);
        }
      });
    };
    publish();
    const resize =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(publish);
    resize?.observe(list);
    // A lista pode crescer (catálogo carregado) sem mudar de caixa.
    const mutate = new MutationObserver(publish);
    mutate.observe(list, { childList: true, subtree: true });
    return () => {
      resize?.disconnect();
      mutate.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [kind, sequence]);

  if (!kind) {
    return null;
  }

  return (
    <div className="menu-window">
      <div className="menu-card" ref={cardRef}>
        {kind === "agent" ? (
          <AgentMenu refreshKey={sequence} />
        ) : (
          <ChannelMenu />
        )}
      </div>
    </div>
  );
}
