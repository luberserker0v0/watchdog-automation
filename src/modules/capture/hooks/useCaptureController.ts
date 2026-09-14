import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getWindowDetails, type WindowDetailInfo } from "../../../api/window";
import type { PipelineTarget } from "../../../api/pipeline";
import { usePreferencesStore, type CaptureMode } from "../../core/stores/usePreferencesStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useWindowStore } from "../stores/useWindowStore";
import { useStreamStore } from "../stores/useStreamStore";
import { refreshOpenPreviewContext } from "../services/previewWindowManager";

export interface UpdateCapturePayload {
  type: "screen" | "window";
  action: "streaming" | "capture" | "sync";
  target: PipelineTarget;
  hwnd: number | null;
}

function matches(pattern: string | undefined, value: string): boolean {
  if (!pattern) return true;
  try {
    return new RegExp(pattern).test(value);
  } catch {
    return false;
  }
}

export function matchWindowByTarget(
  windows: { hwnd?: number; title: string; app_name: string }[],
  target: PipelineTarget,
  hwnd: number | null,
): number {
  if (hwnd) {
    const index = windows.findIndex((window) => window.hwnd === hwnd);
    if (index >= 0) return index;
  }
  if (target.type !== "window") return -1;
  return windows.findIndex((window) =>
    matches(target.title, window.title) && matches(target.process, window.app_name));
}

async function matchWindowByClass(
  windows: { hwnd?: number }[],
  target: PipelineTarget,
): Promise<number> {
  if (target.type !== "window" || !target.class) return -1;
  let details: WindowDetailInfo[];
  try {
    details = await getWindowDetails();
  } catch {
    return -1;
  }
  return windows.findIndex((window) => {
    const detail = details.find((candidate) => candidate.hwnd === window.hwnd);
    return detail ? matches(target.class, detail.class_name) : false;
  });
}

export async function applyCaptureUpdate(payload: UpdateCapturePayload): Promise<void> {
  const { type, action, target, hwnd } = payload;
  if (type !== usePreferencesStore.getState().mode) {
    usePreferencesStore.getState().setMode(type as CaptureMode);
  }

  if (type === "window") {
    await useWindowStore.getState().fetchWindows();
    const windows = useWindowStore.getState().windows;
    let matchedIndex = matchWindowByTarget(windows, target, hwnd);
    if (matchedIndex < 0) matchedIndex = await matchWindowByClass(windows, target);
    if (matchedIndex >= 0) useWindowStore.getState().setSelectedIndex(matchedIndex);
  } else {
    useScreenStore.getState().setSelectedIndex(target.type === "screen" ? target.index ?? 0 : 0);
  }

  if (action === "streaming") {
    await useStreamStore.getState().start(target.type === "window" ? target.capture_area : undefined);
  } else if (action === "sync") {
    await useStreamStore.getState().refreshStatus();
  }

  await refreshOpenPreviewContext();
}

export function useCaptureController(enabled = true) {
  const mode = usePreferencesStore((state) => state.mode);

  useEffect(() => {
    if (!enabled) return;
    void useStreamStore.getState().refreshStatus();
  }, [enabled]);

  useEffect(() => {
    if (!enabled) return;
    if (mode === "screen") void useScreenStore.getState().fetchScreens();
    else void useWindowStore.getState().fetchWindows();
  }, [enabled, mode]);

  useEffect(() => {
    if (!enabled) return;
    let disposed = false;
    let cleanup: (() => void) | null = null;
    void listen<UpdateCapturePayload>("update_capture", (event) => {
      if (!disposed) void applyCaptureUpdate(event.payload);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else cleanup = unlisten;
    });
    return () => {
      disposed = true;
      cleanup?.();
    };
  }, [enabled]);
}
