import { beforeEach, expect, it, vi } from "vitest";

const { check, relaunch } = vi.hoisted(() => ({
  check: vi.fn(),
  relaunch: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("@tauri-apps/plugin-updater", () => ({ check }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn().mockResolvedValue("1.0.0") }));

import { usePipelineStore } from "../../pipeline/runtime/stores/usePipelineStore";
import { useRecorderStore } from "../../recorder/stores/useRecorderStore";
import { resetUpdateStoreForTests, useUpdateStore } from "../stores/useUpdateStore";
import { checkForUpdates, installAvailableUpdate, resetUpdaterServiceForTests } from "./updateService";

function fakeUpdate() {
  return {
    currentVersion: "1.0.0", version: "1.1.0", date: "2026-09-27T00:00:00Z", body: "Safe notes",
    close: vi.fn().mockResolvedValue(undefined),
    download: vi.fn(async (listener: (event: unknown) => void) => {
      listener({ event: "Started", data: { contentLength: 10 } });
      listener({ event: "Progress", data: { chunkLength: 10 } });
      listener({ event: "Finished" });
    }),
    install: vi.fn().mockResolvedValue(undefined),
  };
}

beforeEach(() => {
  resetUpdateStoreForTests();
  resetUpdaterServiceForTests();
  check.mockReset();
  relaunch.mockClear();
  usePipelineStore.setState({ isExecuting: false });
  useRecorderStore.setState({ status: { state: "idle", session_id: null, elapsed_ms: 0, event_count: 0, target_label: null, reason: null, supported: true } });
});

it("coalesces simultaneous checks and records an available update", async () => {
  const update = fakeUpdate();
  check.mockResolvedValue(update);
  const first = checkForUpdates();
  const second = checkForUpdates();
  await Promise.all([first, second]);
  expect(check).toHaveBeenCalledTimes(1);
  expect(useUpdateStore.getState()).toMatchObject({ phase: "available", update: { currentVersion: "1.0.0", version: "1.1.0", notes: "Safe notes" } });
});

it("records a current result and keeps endpoint errors inside updater state", async () => {
  check.mockResolvedValueOnce(null);
  await checkForUpdates();
  expect(useUpdateStore.getState().phase).toBe("current");
  check.mockRejectedValueOnce(new Error("offline"));
  await checkForUpdates();
  expect(useUpdateStore.getState()).toMatchObject({ phase: "error", error: "offline" });
});

it("blocks installation for active work and installs with progress when idle", async () => {
  const update = fakeUpdate();
  check.mockResolvedValue(update);
  await checkForUpdates();
  usePipelineStore.setState({ isExecuting: true });
  await expect(installAvailableUpdate()).rejects.toThrow(/pipeline is active/i);
  usePipelineStore.setState({ isExecuting: false });
  await installAvailableUpdate();
  expect(update.download).toHaveBeenCalledOnce();
  expect(update.install).toHaveBeenCalledWith({ restartAfterInstall: true });
  expect(relaunch).toHaveBeenCalledOnce();
  expect(useUpdateStore.getState()).toMatchObject({ phase: "installing", downloadedBytes: 10, totalBytes: 10 });
});
