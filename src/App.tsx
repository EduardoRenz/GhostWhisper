import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Overlay } from "./features/overlay/Overlay";
import { SettingsPage } from "./features/settings/SettingsPage";
import { SuggestionsWindow } from "./features/suggestions/SuggestionsWindow";
import { PanelWindow } from "./features/panel/PanelWindow";
import { HistoryWindow } from "./features/history/HistoryWindow";
import { DwellPopoverWindow } from "./features/dwell/DwellPopoverWindow";

/** Rotas de janela suportadas pelo GhostWhisper. */
export type Route =
  | "overlay"
  | "settings"
  | "suggestions"
  | "chat"
  | "history"
  | "dwell"
  | "unknown";

/**
 * Resolve a rota a partir do rótulo da janela Tauri.
 *
 * Fora do Tauri (`vite dev` no navegador) aceita
 * `?window=settings|suggestions|chat|history` para desenvolver a UI de cada
 * janela; caso contrário assume o overlay. A janela `chat` é o **painel da
 * sessão** (Transcrição | Resumo) — o resumo deixou de ser janela (BR-033).
 */
export function resolveRoute(): Route {
  try {
    const label = getCurrentWindow().label;
    if (label === "settings") return "settings";
    if (label === "suggestions") return "suggestions";
    if (label === "chat") return "chat";
    if (label === "history") return "history";
    if (label === "dwell") return "dwell";
    if (label === "overlay") return "overlay";
    return "unknown";
  } catch {
    if (typeof window !== "undefined") {
      const target = new URLSearchParams(window.location.search).get("window");
      if (target === "settings") return "settings";
      if (target === "suggestions") return "suggestions";
      if (target === "chat") return "chat";
      if (target === "history") return "history";
      if (target === "dwell") return "dwell";
    }
    return "overlay";
  }
}

export function App() {
  // Resolve já na primeira renderização: janelas ocultas não montam o overlay
  // (e não publicam regiões interativas) antes de trocar de rota.
  const [route, setRoute] = useState<Route>(() => resolveRoute());

  useEffect(() => {
    setRoute(resolveRoute());
  }, []);

  if (route === "overlay") {
    return <Overlay />;
  }

  if (route === "settings") {
    return <SettingsPage />;
  }

  if (route === "suggestions") {
    return <SuggestionsWindow />;
  }

  if (route === "chat") {
    return <PanelWindow />;
  }

  if (route === "history") {
    return <HistoryWindow />;
  }

  if (route === "dwell") {
    return <DwellPopoverWindow />;
  }

  return (
    <main className="app" data-route={route}>
      <h1 className="app__title">GhostWhisper</h1>
      <p className="app__subtitle">Rota: {route}</p>
    </main>
  );
}
