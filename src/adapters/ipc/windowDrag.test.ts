import { beforeEach, describe, expect, it, vi } from "vitest";

const client = vi.hoisted(() => ({
  invokeSafe: vi.fn(async () => undefined),
}));

vi.mock("./client", () => client);

import { startWindowDrag, startWindowResize } from "./windowDrag";

describe("windowDrag", () => {
  beforeEach(() => {
    client.invokeSafe.mockClear();
  });

  it("pede o arraste com timestamp X11 ao core", async () => {
    await startWindowDrag();

    expect(client.invokeSafe).toHaveBeenCalledWith("window_start_drag");
  });

  it("pede o resize na direção escolhida", async () => {
    await startWindowResize("SouthEast");

    expect(client.invokeSafe).toHaveBeenCalledWith("window_start_resize", {
      direction: "SouthEast",
    });
  });
});
