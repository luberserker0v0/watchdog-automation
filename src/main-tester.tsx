import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { initializeI18n } from "./i18n";
import { useDebugStore } from "./modules/capture/stores/useDebugStore";
import { useStreamStore } from "./modules/capture/stores/useStreamStore";
import { useGraphLayoutStore } from "./modules/pipeline/authoring/stores/useGraphLayoutStore";
import { usePreferencesStore } from "./modules/shell/stores/usePreferencesStore";

async function bootstrap() {
  await Promise.all([
    initializeI18n(),
    usePreferencesStore.persist.rehydrate(),
    useDebugStore.persist.rehydrate(),
    useStreamStore.persist.rehydrate(),
    useGraphLayoutStore.persist.rehydrate(),
  ]);
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode><App mode="tester" /></React.StrictMode>,
  );
}

void bootstrap();
