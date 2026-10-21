import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Overlay } from "./features/overlay/Overlay";
import { SettingsPage } from "./features/settings/SettingsPage";
import { SuggestionsWindow } from "./features/suggestions/SuggestionsWindow";
import { PanelWindow } from "./features/panel/PanelWindow";
import { HistoryWindow } from "./features/history/HistoryWindow";
import { DwellPopoverWindow } from "./features/dwell/DwellPopoverWindow";
import { AskWindow } from "./features/ask/AskWindow";
import { CapturePreviewWindow } from "./features/ask/CapturePreviewWindow";
import { SessionEndWindow } from "./features/session-end/SessionEndWindow";
import { useHudOpacity } from "./core/application/useHudOpacity";

/** Rotas de janela suportadas pelo GhostWhisper. */
export type Route =
  | "overlay"
  | "settings"
  | "suggestions"
  | "chat"
  | "history"
  | "dwell"
  | "ask"
  | "capture"
  | "session-end"
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
    if (label === "ask") return "ask";
    if (label === "capture") return "capture";
    if (label === "session-end") return "session-end";
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
    if (target === "ask") return "ask";
    if (target === "capture") return "capture";
    if (target === "session-end") return "session-end";
    }
    return "overlay";
  }
}

export function App() {
  // Opacidade global (BR-062): vale para todas as janelas, inclusive as
  // ocultas, e recebe preview em tempo real do slider de Configurações.
  useHudOpacity();

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

  if (route === "ask") {
    return <AskWindow />;
  }

  if (route === "capture") {
    return <CapturePreviewWindow />;
  }

  if (route === "session-end") {
    return <SessionEndWindow />;
  }

  return (
    <main className="app" data-route={route}>
      <h1 className="app__title">GhostWhisper</h1>
      <p className="app__subtitle">Rota: {route}</p>
    </main>
  );
}
