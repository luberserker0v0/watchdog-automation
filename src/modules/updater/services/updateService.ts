import type { Update } from "@tauri-apps/plugin-updater";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import { appLogger } from "../../observability/services/appLogger";
import { usePipelineStore } from "../../pipeline/runtime/stores/usePipelineStore";
import { useRecorderStore } from "../../recorder/stores/useRecorderStore";
import { useStreamStore } from "../../capture/stores/useStreamStore";
import { useUpdateStore } from "../stores/useUpdateStore";
import {
  activateUpdateInstallationGuard,
  releaseUpdateInstallationGuard,
} from "./runtimeGuard";

let availableUpdate: Update | null = null;
let checkFlight: Promise<void> | null = null;
let installFlight: Promise<void> | null = null;

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function replaceUpdate(next: Update | null): Promise<void> {
  const previous = availableUpdate;
  availableUpdate = next;
  if (previous && previous !== next) {
    await previous.close().catch(() => undefined);
  }
}

export function checkForUpdates(): Promise<void> {
  if (checkFlight) return checkFlight;
  const phase = useUpdateStore.getState().phase;
  if (phase === "downloading" || phase === "installing") return Promise.resolve();

  checkFlight = (async () => {
    useUpdateStore.getState().setState({ phase: "checking", error: null });
    try {
      const currentVersion = await getVersion();
      useUpdateStore.getState().setState({ currentVersion });
      const result = await check({ timeout: 20_000 });
      const checkedAt = Date.now();
      await replaceUpdate(result);
      if (!result) {
        useUpdateStore.getState().setState({
          phase: "current",
          update: null,
          downloadedBytes: 0,
          totalBytes: null,
          lastCheckTime: checkedAt,
          error: null,
        });
        return;
      }
      useUpdateStore.getState().setState({
        phase: "available",
        update: {
          currentVersion: result.currentVersion,
          version: result.version,
          publicationDate: result.date ?? null,
          notes: result.body ?? "",
        },
        currentVersion: result.currentVersion,
        downloadedBytes: 0,
        totalBytes: null,
        lastCheckTime: checkedAt,
        error: null,
      });
    } catch (error) {
      const message = errorText(error);
      appLogger.warn(`[updater] update check failed: ${message}`);
      useUpdateStore.getState().setState({
        phase: "error",
        lastCheckTime: Date.now(),
        error: message,
      });
    }
  })().finally(() => { checkFlight = null; });
  return checkFlight;
}

export function runtimeBusyReason(): "pipeline" | "recorder" | null {
  if (usePipelineStore.getState().isExecuting) return "pipeline";
  const recorder = useRecorderStore.getState().status.state;
  return recorder === "recording" || recorder === "paused" || recorder === "stopping"
    ? "recorder"
    : null;
}

export function installAvailableUpdate(): Promise<void> {
  if (installFlight) return installFlight;
  const update = availableUpdate;
  if (!update) return Promise.reject(new Error("No application update is available."));
  const busy = runtimeBusyReason();
  if (busy) return Promise.reject(new Error(`Cannot install while the ${busy} is active.`));
  if (!activateUpdateInstallationGuard()) {
    return Promise.reject(new Error("An application update is already being installed."));
  }

  installFlight = (async () => {
    try {
      const stream = useStreamStore.getState();
      if (stream.isStreaming && stream.captureOwner === "manual") await stream.stop();
      useUpdateStore.getState().setState({
        phase: "downloading",
        downloadedBytes: 0,
        totalBytes: null,
        error: null,
      });
      await update.download((event) => {
        if (event.event === "Started") {
          useUpdateStore.getState().setState({ totalBytes: event.data.contentLength ?? null });
        } else if (event.event === "Progress") {
          useUpdateStore.getState().setState({
            downloadedBytes: useUpdateStore.getState().downloadedBytes + event.data.chunkLength,
          });
        } else {
          useUpdateStore.getState().setState({ phase: "installing" });
        }
      }, { timeout: 120_000 });
      useUpdateStore.getState().setState({ phase: "installing" });
      await update.install({ restartAfterInstall: true });
      // Windows exits during install. Other supported platforms need an explicit relaunch.
      await relaunch();
    } catch (error) {
      const message = errorText(error);
      appLogger.error(`[updater] installation failed: ${message}`);
      releaseUpdateInstallationGuard();
      useUpdateStore.getState().setState({ phase: "available", error: message });
      throw error;
    }
  })().finally(() => { installFlight = null; });
  return installFlight;
}

export async function disposeUpdaterResource(): Promise<void> {
  await replaceUpdate(null);
}

export function resetUpdaterServiceForTests(): void {
  availableUpdate = null;
  checkFlight = null;
  installFlight = null;
  releaseUpdateInstallationGuard();
}
