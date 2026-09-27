import React from "react";
import ReactDOM from "react-dom/client";
import SplashScreen from "./modules/shell/views/SplashScreen";
import { LANGUAGE_STORAGE_KEY, type AppLanguage } from "./i18n";
import { readBootstrapValue } from "./modules/shell/services/persistence";

async function bootstrap() {
  const [languageValue, settingsValue] = await Promise.all([
    readBootstrapValue(LANGUAGE_STORAGE_KEY),
    readBootstrapValue("watchdog:settings"),
  ]);
  const language: AppLanguage = languageValue === "zh-TW" ? "zh-TW" : "en";
  let recognitionDevice: "cpu" | "directml" = "cpu";
  try {
    recognitionDevice = JSON.parse(settingsValue ?? "null")?.state?.recognitionDevice === "directml"
      ? "directml"
      : "cpu";
  } catch { /* Corrupt settings use safe defaults. */ }
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <SplashScreen language={language} recognitionDevice={recognitionDevice} />
    </React.StrictMode>,
  );
}

void bootstrap();
