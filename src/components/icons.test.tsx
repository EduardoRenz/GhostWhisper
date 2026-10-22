import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  ChevronDownIcon,
  CollapseIcon,
  EyeIcon,
  EyeOffIcon,
  GearIcon,
  GripIcon,
  HistoryIcon,
  InfoIcon,
  LanguageIcon,
  PencilIcon,
  PlusIcon,
  SparkleIcon,
  StarIcon,
  SummaryIcon,
  MonitorIcon,
  TranscriptIcon,
  VideoCameraIcon,
  WindowIcon,
} from "./icons";
import { ShortcutBadges } from "./ShortcutBadges";

const ALL_ICONS = {
  GearIcon,
  TranscriptIcon,
  SummaryIcon,
  HistoryIcon,
  EyeIcon,
  EyeOffIcon,
  ChevronDownIcon,
  CollapseIcon,
  GripIcon,
  LanguageIcon,
  SparkleIcon,
  VideoCameraIcon,
  MonitorIcon,
  WindowIcon,
  PencilIcon,
  StarIcon,
  PlusIcon,
  InfoIcon,
};

describe("ícones", () => {
  it("todos desenham um svg decorativo (sem foco, sem rótulo próprio)", () => {
    for (const [name, Icon] of Object.entries(ALL_ICONS)) {
      const { container, unmount } = render(<Icon />);
      const svg = container.querySelector("svg");
      expect(svg, name).not.toBeNull();
      expect(svg?.getAttribute("aria-hidden"), name).toBe("true");
      expect(svg?.getAttribute("focusable"), name).toBe("false");
      expect(svg?.getAttribute("viewBox"), name).toBe("0 0 24 24");
      unmount();
    }
  });

  it("respeita o tamanho informado", () => {
    const { container } = render(<GearIcon size={24} />);
    const svg = container.querySelector("svg");
    expect(svg?.getAttribute("width")).toBe("24");
    expect(svg?.getAttribute("height")).toBe("24");
  });
});

describe("ShortcutBadges", () => {
  it("desenha um badge por token do atalho", () => {
    const { container } = render(
      <ShortcutBadges accelerator="CommandOrControl+Shift+S" />,
    );
    const badges = container.querySelectorAll("kbd");
    expect([...badges].map((badge) => badge.textContent)).toEqual([
      "Ctrl",
      "Shift",
      "S",
    ]);
  });

  it("não desenha nada sem atalho", () => {
    const { container } = render(<ShortcutBadges accelerator="" />);
    expect(container.querySelector(".shortcut-badges")).toBeNull();
  });
});
