import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Overlay } from "./features/overlay/Overlay";
import { SettingsPage } from "./features/settings/SettingsPage";
import { SummaryWindow } from "./features/summary/SummaryWindow";
import { SuggestionsWindow } from "./features/suggestions/SuggestionsWindow";
import { ChatWindow } from "./features/chat/ChatWindow";
import { HistoryWindow } from "./features/history/HistoryWindow";

/** Rotas de janela suportadas pelo GhostWhisper. */
export type Route =
  | "overlay"
  | "settings"
  | "summary"
  | "suggestions"
  | "chat"
  | "history"
  | "unknown";

/**
 * Resolve a rota a partir do rótulo da janela Tauri.
 *
 * Fora do Tauri (`vite dev` no navegador) aceita
 * `?window=settings|summary|suggestions|chat|history` para desenvolver a UI de
 * cada janela; caso contrário assume o overlay.
 */
export function resolveRoute(): Route {
  try {
    const label = getCurrentWindow().label;
    if (label === "settings") return "settings";
    if (label === "summary") return "summary";
    if (label === "suggestions") return "suggestions";
    if (label === "chat") return "chat";
    if (label === "history") return "history";
    if (label === "overlay") return "overlay";
    return "unknown";
  } catch {
    if (typeof window !== "undefined") {
      const target = new URLSearchParams(window.location.search).get("window");
      if (target === "settings") return "settings";
      if (target === "summary") return "summary";
      if (target === "suggestions") return "suggestions";
      if (target === "chat") return "chat";
      if (target === "history") return "history";
    }
    return "overlay";
  }
}

export function App() {
  const [route, setRoute] = useState<Route>("overlay");

  useEffect(() => {
    setRoute(resolveRoute());
  }, []);

  if (route === "overlay") {
    return <Overlay />;
  }

  if (route === "settings") {
    return <SettingsPage />;
  }

  if (route === "summary") {
    return <SummaryWindow />;
  }

  if (route === "suggestions") {
    return <SuggestionsWindow />;
  }

  if (route === "chat") {
    return <ChatWindow />;
  }

  if (route === "history") {
    return <HistoryWindow />;
  }

  return (
    <main className="app" data-route={route}>
      <h1 className="app__title">GhostWhisper</h1>
      <p className="app__subtitle">Rota: {route}</p>
    </main>
  );
}
