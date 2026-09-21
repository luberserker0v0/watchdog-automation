import { beforeEach, expect, it, vi } from "vitest";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useStreamStore } from "../stores/useStreamStore";
import { applyCaptureUpdate, matchWindowByTarget } from "./useCaptureController";

const { refreshPreview } = vi.hoisted(() => ({
  refreshPreview: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../services/previewWindowManager", () => ({
  refreshOpenPreviewContext: refreshPreview,
}));

beforeEach(() => {
  usePreferencesStore.setState({ mode: "window", capturePanelVisible: false });
  useScreenStore.setState({ selectedIndex: null });
  useStreamStore.setState({
    start: vi.fn().mockResolvedValue(undefined),
    refreshStatus: vi.fn().mockResolvedValue(undefined),
  });
});

it("matches a window by HWND first and rejects invalid fallback regexes", () => {
  const windows = [
    { hwnd: 10, title: "WatchDog Target", app_name: "watch-dog-target" },
    { hwnd: 20, title: "Other", app_name: "other" },
  ];
  expect(matchWindowByTarget(windows, { type: "window", title: "^Other$" }, 10)).toBe(0);
  expect(matchWindowByTarget(windows, { type: "window", title: "^Other$" }, null)).toBe(1);
  expect(matchWindowByTarget(windows, { type: "window", title: "[" }, null)).toBe(-1);
});

it("processes capture updates while the Capture panel is hidden", async () => {
  const start = useStreamStore.getState().start;
  await applyCaptureUpdate({
    type: "screen",
    action: "streaming",
    target: { type: "screen", index: 2 },
    hwnd: null,
  });

  expect(usePreferencesStore.getState()).toMatchObject({ mode: "screen", capturePanelVisible: false });
  expect(useScreenStore.getState().selectedIndex).toBe(2);
  expect(start).toHaveBeenCalledWith(undefined);
  expect(refreshPreview).toHaveBeenCalledOnce();
});
