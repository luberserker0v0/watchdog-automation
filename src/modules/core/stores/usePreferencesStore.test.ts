import { beforeEach, expect, it } from "vitest";
import { usePreferencesStore } from "./usePreferencesStore";

beforeEach(() => {
  usePreferencesStore.setState({ mode: "screen", saveToDisk: false, showFps: false, captureEngine: "graphics", windowCaptureArea: "frame" });
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
});
