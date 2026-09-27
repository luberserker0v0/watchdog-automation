import { isTauri } from "@tauri-apps/api/core";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { readPersistedValue, writePersistedValue } from "../modules/shell/services/persistence";
import { resources } from "./resources";

export type AppLanguage = "en" | "zh-TW";
export const LANGUAGE_STORAGE_KEY = "watchdog:language";

let initialization: Promise<void> | null = null;
let usePluginPersistence = isTauri();

function normalizeLanguage(value: unknown): AppLanguage {
  return value === "zh-TW" ? "zh-TW" : "en";
}

function configure(language: AppLanguage): Promise<void> {
  if (i18n.isInitialized) return Promise.resolve();
  document.documentElement.lang = language;
  return i18n.use(initReactI18next).init({
    resources,
    lng: language,
    fallbackLng: "en",
    supportedLngs: ["en", "zh-TW"],
    interpolation: { escapeValue: false },
    react: { useSuspense: false },
  }).then(() => undefined);
}

export function initializeI18n(): Promise<void> {
  initialization ??= (async () => {
    const saved = isTauri()
      ? await readPersistedValue(LANGUAGE_STORAGE_KEY)
      : localStorage.getItem(LANGUAGE_STORAGE_KEY);
    await configure(normalizeLanguage(saved));
  })();
  return initialization;
}

export function initializeStandaloneI18n(): Promise<void> {
  usePluginPersistence = false;
  return configure(normalizeLanguage(localStorage.getItem(LANGUAGE_STORAGE_KEY)));
}

i18n.on("languageChanged", (next) => {
  const normalized = normalizeLanguage(next);
  document.documentElement.lang = normalized;
  if (usePluginPersistence) void writePersistedValue(LANGUAGE_STORAGE_KEY, normalized);
  else localStorage.setItem(LANGUAGE_STORAGE_KEY, normalized);
});

// Browser previews and tests keep synchronous initialization semantics.
if (!isTauri()) {
  void configure(normalizeLanguage(localStorage.getItem(LANGUAGE_STORAGE_KEY)));
  initialization = Promise.resolve();
}

export async function changeAppLanguage(language: AppLanguage) {
  await initializeI18n();
  await i18n.changeLanguage(language);
}

export default i18n;
