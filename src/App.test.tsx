import { describe, expect, it, vi } from "vitest";

const windowLabel = vi.hoisted(() => ({ current: "overlay" }));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ label: windowLabel.current }),
}));

import { resolveRoute } from "./App";

describe("resolveRoute", () => {
  it("mapeia o label de cada janela para a rota", () => {
    windowLabel.current = "overlay";
    expect(resolveRoute()).toBe("overlay");

    windowLabel.current = "settings";
    expect(resolveRoute()).toBe("settings");

    windowLabel.current = "suggestions";
    expect(resolveRoute()).toBe("suggestions");

    windowLabel.current = "chat";
    expect(resolveRoute()).toBe("chat");

    windowLabel.current = "history";
    expect(resolveRoute()).toBe("history");

    windowLabel.current = "desconhecida";
    expect(resolveRoute()).toBe("unknown");
  });
});
