import { listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { PhysicalSize } from "@tauri-apps/api/window";
import type { PipelineTarget } from "../../../api/pipeline";
import { usePipelineStore } from "../../pipeline/stores/usePipelineStore";
import { normalizePreviewWindowSize, usePreferencesStore, type CaptureArea } from "../../core/stores/usePreferencesStore";
import { useUiStore } from "../../core/stores/useUiStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import { useOverlayStore } from "../stores/useOverlayStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useWindowStore } from "../stores/useWindowStore";
import { escapeExactRegex } from "../components/tools/nodeEmulatorModel";

const PREVIEW_LABEL = "capture-preview";
const RESIZE_DEBOUNCE_MS = 180;

interface TargetInfoPayload {
  targetName: string;
  mode: "screen" | "window";
  engine: string;
  statusClass: "idle" | "streaming" | "captured";
  statusLabel: string;
  targetIndex: number | null;
  captureArea?: CaptureArea;
  target?: PipelineTarget;
  expectedSize?: [number, number] | null;
  frameSize?: [number, number] | null;
  clientSize?: [number, number] | null;
}

let activePreviewWindow: WebviewWindow | null = null;
let trackedWindow: WebviewWindow | null = null;
let cleanupFunctions: Array<() => void> = [];
let resizeTimer: ReturnType<typeof setTimeout> | null = null;
let pendingReadyCleanup: (() => void) | null = null;

export function logicalPreviewSize(size: Pick<PhysicalSize, "width" | "height">, scaleFactor: number): [number, number] {
  const safeScale = Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1;
  return normalizePreviewWindowSize([size.width / safeScale, size.height / safeScale]);
}

function clearTracking(window: WebviewWindow) {
  if (trackedWindow !== window && activePreviewWindow !== window) return;
  if (resizeTimer) clearTimeout(resizeTimer);
  resizeTimer = null;
  pendingReadyCleanup?.();
  pendingReadyCleanup = null;
  cleanupFunctions.splice(0).forEach((cleanup) => cleanup());
  if (trackedWindow === window) trackedWindow = null;
  if (activePreviewWindow === window) activePreviewWindow = null;
  useUiStore.getState().setPreviewOpen(false);
}

async function trackWindow(window: WebviewWindow) {
  if (trackedWindow === window) return;
  if (trackedWindow) clearTracking(trackedWindow);
  trackedWindow = window;

  const register = async (promise: Promise<() => void>) => {
    const cleanup = await promise;
    if (trackedWindow === window) cleanupFunctions.push(cleanup);
    else cleanup();
  };
  await Promise.all([
    register(window.once("tauri://destroyed", () => clearTracking(window))),
    register(window.once("tauri://error", (event) => {
      console.error("Preview window error:", event);
      clearTracking(window);
    })),
    register(window.onResized((event) => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        void window.scaleFactor()
          .then((scaleFactor) => usePreferencesStore.getState().setPreviewWindowSize(logicalPreviewSize(event.payload, scaleFactor)))
          .catch((error) => console.warn("Failed to persist Preview size:", error));
      }, RESIZE_DEBOUNCE_MS);
    })),
  ]);
}

function computeTargetInfo(): TargetInfoPayload {
  const preferences = usePreferencesStore.getState();
  const stream = useStreamStore.getState();
  const mode = preferences.mode;
  const frame = useCaptureFrameStore.getState().frames[mode];
  const activePipelineId = usePipelineStore.getState().activeId;
  const activeTarget = activePipelineId
    ? usePipelineStore.getState().pipelinesById[activePipelineId]?.graph?.target
    : undefined;
  const imageDimensions = useOverlayStore.getState().imageDimensions;
  const frameSize = stream.frameSize
    ?? (imageDimensions ? [imageDimensions.width, imageDimensions.height] as [number, number] : null);
  let targetName = "未選取";
  let targetIndex: number | null = null;
  let captureArea: CaptureArea | undefined;
  let target: PipelineTarget | undefined;

  if (mode === "screen") {
    const selectedIndex = useScreenStore.getState().selectedIndex;
    const screen = selectedIndex === null
      ? undefined
      : useScreenStore.getState().screens.find((candidate) => candidate.index === selectedIndex);
    if (screen) {
      targetIndex = screen.index;
      target = { type: "screen", index: screen.index };
      targetName = `Screen ${screen.index} - ${screen.width}×${screen.height}${screen.is_primary ? " (主螢幕)" : ""}`;
    }
  } else {
    const selectedIndex = useWindowStore.getState().selectedIndex;
    const window = selectedIndex === null ? undefined : useWindowStore.getState().windows[selectedIndex];
    if (window) {
      targetIndex = window.hwnd ?? null;
      captureArea = preferences.windowCaptureArea;
      target = { type: "window", title: escapeExactRegex(window.title), capture_area: captureArea };
      targetName = window.app_name || window.title || "未知";
    }
  }

  const captured = frame.captureBuf !== null;
  return {
    targetName,
    mode,
    engine: preferences.captureEngine,
    statusClass: stream.isStreaming ? "streaming" : captured ? "captured" : "idle",
    statusLabel: stream.isStreaming ? "串流中" : captured ? "已截圖" : "閒置",
    targetIndex,
    captureArea,
    target,
    expectedSize: activeTarget?.type === "window" ? activeTarget.window_size ?? null : null,
    frameSize,
    clientSize: stream.clientSize
      ?? (mode === "window" && preferences.windowCaptureArea === "client" ? frameSize : null),
  };
}

async function sendPreviewContext(window: WebviewWindow) {
  const mode = usePreferencesStore.getState().mode;
  await window.emit("preview-target-info", computeTargetInfo());
  if (!useStreamStore.getState().isStreaming) {
    const bytes = useCaptureFrameStore.getState().frames[mode].captureBuf;
    if (bytes) await window.emit("preview-image", { bytes, mode });
  }
}

export async function refreshOpenPreviewContext(): Promise<void> {
  const window = activePreviewWindow ?? await WebviewWindow.getByLabel(PREVIEW_LABEL);
  if (!window) return;
  activePreviewWindow = window;
  await trackWindow(window);
  await sendPreviewContext(window);
}

export async function openOrFocusPreview(): Promise<void> {
  let window = activePreviewWindow ?? await WebviewWindow.getByLabel(PREVIEW_LABEL);
  if (window) {
    activePreviewWindow = window;
    try {
      await trackWindow(window);
      useUiStore.getState().setPreviewOpen(true);
      useUiStore.getState().setCapturePanelOpen(false);
      await sendPreviewContext(window);
      await window.show();
      await window.setFocus();
    } catch (error) {
      clearTracking(window);
      throw error;
    }
    return;
  }

  const [width, height] = usePreferencesStore.getState().previewWindowSize;
  pendingReadyCleanup?.();
  pendingReadyCleanup = await listen("preview-ready", () => {
    pendingReadyCleanup?.();
    pendingReadyCleanup = null;
    if (activePreviewWindow) void sendPreviewContext(activePreviewWindow);
  });

  window = new WebviewWindow(PREVIEW_LABEL, {
    url: "index.html#preview",
    title: usePreferencesStore.getState().mode === "screen" ? "螢幕預覽" : "視窗預覽",
    width,
    height,
    minWidth: 600,
    minHeight: 400,
    center: true,
    decorations: false,
  });
  activePreviewWindow = window;
  useUiStore.getState().setPreviewOpen(true);
  useUiStore.getState().setCapturePanelOpen(false);
  try {
    await trackWindow(window);
  } catch (error) {
    clearTracking(window);
    throw error;
  }
}
