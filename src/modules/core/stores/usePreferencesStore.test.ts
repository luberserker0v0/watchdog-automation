import { beforeEach, expect, it } from "vitest";
import { normalizePreviewWindowSize, usePreferencesStore } from "./usePreferencesStore";

beforeEach(() => {
  usePreferencesStore.setState({ mode: "screen", saveToDisk: false, showFps: false, captureEngine: "graphics", windowCaptureArea: "frame", capturePanelVisible: true, previewWindowSize: [1200, 800] });
});

it("persists capture visibility and a normalized logical Preview size", () => {
  usePreferencesStore.getState().setCapturePanelVisible(false);
  usePreferencesStore.getState().setPreviewWindowSize([500.4, 300.2]);
  const persisted = JSON.parse(localStorage.getItem("watchdog:settings")!);
  expect(persisted.state).toMatchObject({ capturePanelVisible: false, previewWindowSize: [600, 400] });
  expect(normalizePreviewWindowSize([1400.2, 900.7])).toEqual([1400, 901]);
  expect(normalizePreviewWindowSize("bad")).toEqual([1200, 800]);
});

it("keeps the existing settings storage key and persists only preferences", () => {
  usePreferencesStore.getState().setMode("window");
  usePreferencesStore.getState().setShowFps(true);
  usePreferencesStore.getState().setWindowCaptureArea("client");
  const persisted = JSON.parse(localStorage.getItem("watchdog:settings")!);
  expect(persisted.state).toMatchObject({ mode: "window", showFps: true, windowCaptureArea: "client" });
  expect(persisted.state).not.toHaveProperty("activeView");
  expect(persisted.state).not.toHaveProperty("previewOpen");
});

it("hydrates legacy settings without a capture area as frame", async () => {
  usePreferencesStore.setState({ windowCaptureArea: "frame" });
  localStorage.setItem(
    "watchdog:settings",
    JSON.stringify({ state: { mode: "window", captureEngine: "gdi" }, version: 0 }),
  );
  await usePreferencesStore.persist.rehydrate();
  expect(usePreferencesStore.getState().windowCaptureArea).toBe("frame");
  expect(usePreferencesStore.getState().capturePanelVisible).toBe(true);
  expect(usePreferencesStore.getState().previewWindowSize).toEqual([1200, 800]);
});
