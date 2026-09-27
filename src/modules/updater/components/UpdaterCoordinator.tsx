import { useEffect } from "react";
import { getStartupProgress } from "../../../api/startup";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { checkForUpdates, disposeUpdaterResource } from "../services/updateService";

const wait = (milliseconds: number) => new Promise<void>((resolve) => setTimeout(resolve, milliseconds));

async function waitForStartup(): Promise<void> {
  for (;;) {
    const progress = await getStartupProgress();
    if (progress.complete) return;
    await wait(100);
  }
}

export default function UpdaterCoordinator() {
  useEffect(() => {
    let active = true;
    void waitForStartup()
      .then(() => {
        if (active && usePreferencesStore.getState().automaticUpdateChecks) {
          return checkForUpdates();
        }
      })
      .catch(() => undefined);
    return () => {
      active = false;
      void disposeUpdaterResource();
    };
  }, []);
  return null;
}
