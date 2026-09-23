import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { resources } from "./resources";

export type AppLanguage = "en" | "zh-TW";
export const LANGUAGE_STORAGE_KEY = "watchdog:language";

const saved = localStorage.getItem(LANGUAGE_STORAGE_KEY);
const language: AppLanguage = saved === "zh-TW" ? "zh-TW" : "en";

void i18n.use(initReactI18next).init({
  resources,
  lng: language,
  fallbackLng: "en",
  supportedLngs: ["en", "zh-TW"],
  interpolation: { escapeValue: false },
  react: { useSuspense: false },
});

i18n.on("languageChanged", (next) => {
  const normalized: AppLanguage = next === "zh-TW" ? "zh-TW" : "en";
  localStorage.setItem(LANGUAGE_STORAGE_KEY, normalized);
  document.documentElement.lang = normalized;
});
document.documentElement.lang = language;

export async function changeAppLanguage(language: AppLanguage) {
  await i18n.changeLanguage(language);
}

export default i18n;
