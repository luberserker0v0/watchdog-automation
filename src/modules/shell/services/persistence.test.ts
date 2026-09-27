import { beforeEach, expect, it, vi } from "vitest";

const { values, nativeStore, loadStore } = vi.hoisted(() => {
  const values = new Map<string, unknown>();
  const nativeStore = {
    get: vi.fn((key: string) => Promise.resolve(values.get(key) ?? null)),
    set: vi.fn((key: string, value: unknown) => { values.set(key, value); return Promise.resolve(); }),
    delete: vi.fn((key: string) => { values.delete(key); return Promise.resolve(true); }),
    save: vi.fn(() => Promise.resolve()),
  };
  return {
    values,
    nativeStore,
    loadStore: vi.fn(() => Promise.resolve(nativeStore)),
  };
});
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true }));
vi.mock("@tauri-apps/plugin-store", () => ({ load: loadStore }));
vi.mock("@tauri-apps/plugin-log", () => ({
  info: vi.fn(() => Promise.resolve()),
  warn: vi.fn(() => Promise.resolve()),
  error: vi.fn(() => Promise.resolve()),
}));

beforeEach(() => {
  values.clear();
  localStorage.clear();
  vi.clearAllMocks();
  vi.resetModules();
});

it("moves a legacy localStorage value only after the native Store saves", async () => {
  const { watchdogStateStorage } = await import("./persistence");
  localStorage.setItem("watchdog:settings", "persisted-settings");
  await expect(watchdogStateStorage.getItem("watchdog:settings")).resolves.toBe("persisted-settings");
  expect(loadStore).toHaveBeenCalled();
  expect(nativeStore.get).toHaveBeenCalledWith("watchdog:settings");
  expect(nativeStore.set).toHaveBeenCalledWith("watchdog:settings", "persisted-settings");
  expect(nativeStore.save).toHaveBeenCalledOnce();
  expect(localStorage.getItem("watchdog:settings")).toBeNull();
});

it("lets the read-only splash fall back without migrating or deleting", async () => {
  const { readBootstrapValue } = await import("./persistence");
  localStorage.setItem("watchdog:language", "zh-TW");
  await expect(readBootstrapValue("watchdog:language")).resolves.toBe("zh-TW");
  expect(nativeStore.set).not.toHaveBeenCalled();
  expect(localStorage.getItem("watchdog:language")).toBe("zh-TW");
});

it("keeps legacy values available when a corrupted native Store cannot load", async () => {
  loadStore.mockRejectedValueOnce(new Error("invalid store JSON"));
  const { readBootstrapValue } = await import("./persistence");
  localStorage.setItem("watchdog:language", "zh-TW");
  await expect(readBootstrapValue("watchdog:language")).resolves.toBe("zh-TW");
  expect(localStorage.getItem("watchdog:language")).toBe("zh-TW");
});
