import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Overlay } from "./features/overlay/Overlay";
import { SettingsPage } from "./features/settings/SettingsPage";

/** Rotas de janela suportadas pelo GhostWhisper. */
export type Route = "overlay" | "settings" | "unknown";

/**
 * Resolve a rota a partir do rótulo da janela Tauri.
 *
 * Fora do Tauri (`vite dev` no navegador) aceita `?window=settings` para
 * desenvolver a UI de configurações; caso contrário assume o overlay.
 */
export function resolveRoute(): Route {
  try {
    const label = getCurrentWindow().label;
    if (label === "settings") return "settings";
    if (label === "overlay") return "overlay";
    return "unknown";
  } catch {
    if (
      typeof window !== "undefined" &&
      new URLSearchParams(window.location.search).get("window") === "settings"
    ) {
      return "settings";
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

  return (
    <main className="app" data-route={route}>
      <h1 className="app__title">GhostWhisper</h1>
      <p className="app__subtitle">Rota: {route}</p>
    </main>
  );
}
