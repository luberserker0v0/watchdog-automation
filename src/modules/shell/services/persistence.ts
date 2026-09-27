import { isTauri } from "@tauri-apps/api/core";
import { load, type Store } from "@tauri-apps/plugin-store";
import { createJSONStorage, type StateStorage } from "zustand/middleware";
import { appLogger } from "../../observability/services/appLogger";

export const SETTINGS_STORE_FILE = "watchdog.settings.json";

let storePromise: Promise<Store | null> | null = null;

function browserStorage(): Storage | null {
  return typeof localStorage === "undefined" ? null : localStorage;
}

function nativeStore(): Promise<Store | null> {
  storePromise ??= load(SETTINGS_STORE_FILE, { autoSave: 100 }).catch((error) => {
    appLogger.error("[settings] unable to load the native Store; using safe defaults", error);
    return null;
  });
  return storePromise;
}

async function readNativeOrMigrate(name: string): Promise<string | null> {
  const store = await nativeStore();
  if (!store) return browserStorage()?.getItem(name) ?? null;
  let stored: unknown;
  try {
    stored = await store.get<unknown>(name);
  } catch (error) {
    appLogger.error(`[settings] unable to read Store key ${name}`, error);
    return browserStorage()?.getItem(name) ?? null;
  }
  if (typeof stored === "string") return stored;
  if (stored !== null && stored !== undefined) return JSON.stringify(stored);

  const legacy = browserStorage()?.getItem(name) ?? null;
  if (legacy === null) return null;
  await store.set(name, legacy);
  await store.save();
  browserStorage()?.removeItem(name);
  return legacy;
}

export const watchdogStateStorage: StateStorage = {
  getItem: async (name) => {
    if (!isTauri()) return browserStorage()?.getItem(name) ?? null;
    return readNativeOrMigrate(name);
  },
  setItem: async (name, value) => {
    if (!isTauri()) {
      browserStorage()?.setItem(name, value);
      return;
    }
    const store = await nativeStore();
    if (!store) return;
    await store.set(name, value);
  },
  removeItem: async (name) => {
    if (!isTauri()) {
      browserStorage()?.removeItem(name);
      return;
    }
    const store = await nativeStore();
    if (!store) return;
    await store.delete(name);
    browserStorage()?.removeItem(name);
  },
};

export const watchdogJsonStorage = createJSONStorage(() => watchdogStateStorage);

export async function readPersistedValue(name: string): Promise<string | null> {
  return watchdogStateStorage.getItem(name);
}

/** Read-only bootstrap path for the splash window, which has no Store write permission. */
export async function readBootstrapValue(name: string): Promise<string | null> {
  if (!isTauri()) return browserStorage()?.getItem(name) ?? null;
  const store = await nativeStore();
  if (!store) return browserStorage()?.getItem(name) ?? null;
  let stored: unknown;
  try {
    stored = await store.get<unknown>(name);
  } catch (error) {
    appLogger.error(`[settings] unable to read bootstrap Store key ${name}`, error);
    return browserStorage()?.getItem(name) ?? null;
  }
  if (typeof stored === "string") return stored;
  if (stored !== null && stored !== undefined) return JSON.stringify(stored);
  return browserStorage()?.getItem(name) ?? null;
}

export async function writePersistedValue(name: string, value: string): Promise<void> {
  await watchdogStateStorage.setItem(name, value);
}

export function resetPersistenceForTests(): void {
  storePromise = null;
}
