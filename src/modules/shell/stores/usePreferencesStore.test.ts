import { beforeEach, expect, it } from "vitest";
import { normalizePreviewWindowSize, usePreferencesStore } from "./usePreferencesStore";

beforeEach(() => {
  usePreferencesStore.setState({ mode: "screen", saveToDisk: false, showFps: false, captureEngine: "graphics", windowCaptureArea: "frame", patrolEnabled: false, patrolKHops: 1, patrolThreshold: 0.85, patrolMinMargin: 0.1, capturePanelVisible: true, previewWindowSize: [1200, 800] });
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

it("keeps save-to-disk and debug images off on every window open", async () => {
  usePreferencesStore.getState().setSaveToDisk(true);
  usePreferencesStore.getState().setSaveDebugImages(true);
  const persisted = JSON.parse(localStorage.getItem("watchdog:settings")!);
  expect(persisted.state).not.toHaveProperty("saveToDisk");
  expect(persisted.state).not.toHaveProperty("saveDebugImages");
  // Stale true values written by older versions must not rehydrate.
  localStorage.setItem(
    "watchdog:settings",
    JSON.stringify({ state: { saveToDisk: true, saveDebugImages: true }, version: 0 }),
  );
  await usePreferencesStore.persist.rehydrate();
  expect(usePreferencesStore.getState().saveToDisk).toBe(false);
  expect(usePreferencesStore.getState().saveDebugImages).toBe(false);
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

it("persists LLM endpoint and model without storing the API key", async () => {
  const settings = usePreferencesStore.getState();
  settings.setLlmEndpoint("http://localhost:8000/v1/chat/completions");
  settings.setLlmModel("vision-model");
  settings.setLlmApiKey("temporary-secret");
  const persisted = JSON.parse(localStorage.getItem("watchdog:settings")!);
  expect(persisted.state).toMatchObject({
    llmEndpoint: "http://localhost:8000/v1/chat/completions",
    llmModel: "vision-model",
  });
  expect(persisted.state).not.toHaveProperty("llmApiKey");
  await usePreferencesStore.persist.rehydrate();
  expect(usePreferencesStore.getState().llmApiKey).toBe("");
});

it("persists patrol enablement and validates restored search options", async () => {
  const settings = usePreferencesStore.getState();
  settings.setPatrolEnabled(true);
  settings.setPatrolKHops(2);
  settings.setPatrolThreshold(0.9);
  settings.setPatrolMinMargin(0.2);
  const persisted = JSON.parse(localStorage.getItem("watchdog:settings")!);
  expect(persisted.state).toMatchObject({ patrolEnabled: true, patrolKHops: 2, patrolThreshold: 0.9, patrolMinMargin: 0.2 });
  expect(persisted.state).not.toHaveProperty("patrolMode");

  localStorage.setItem("watchdog:settings", JSON.stringify({ state: { patrolMode: "invalid", patrolKHops: 99, patrolThreshold: 3, patrolMinMargin: -1 }, version: 0 }));
  await usePreferencesStore.persist.rehydrate();
  expect(usePreferencesStore.getState()).toMatchObject({ patrolEnabled: false, patrolKHops: 4, patrolThreshold: 1, patrolMinMargin: 0 });
});
