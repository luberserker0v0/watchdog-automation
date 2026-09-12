import { create } from "zustand";
import { persist } from "zustand/middleware";
import { listen } from "@tauri-apps/api/event";
import { startStream, stopStream, getStreamStatus } from "../../../api/stream";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useScreenStore } from "./useScreenStore";
import { useWindowStore } from "./useWindowStore";
import { useCaptureFrameStore } from "./useCaptureFrameStore";
import type { CaptureArea } from "../../core/stores/usePreferencesStore";

interface StreamState {
  isStreaming: boolean;
  fps: number;
  actualFps: number;
  startTime: number | null;
  elapsedTime: string;
  setFps: (fps: number) => void;
  start: (captureAreaOverride?: CaptureArea) => Promise<void>;
  stop: () => Promise<void>;
  refreshStatus: () => Promise<void>;
}

let unlistenStream: (() => void) | null = null;
let elapsedTimer: ReturnType<typeof setInterval> | null = null;
let fpsTimer: ReturnType<typeof setInterval> | null = null;
let frameCount = 0;
let lastFpsTime = 0;

function formatElapsed(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  if (h > 0) {
    return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

function startFpsTimer(set: (partial: Partial<StreamState>) => void) {
  if (fpsTimer) return;
  frameCount = 0;
  lastFpsTime = Date.now();
  fpsTimer = setInterval(() => {
    const now = Date.now();
    const elapsed = (now - lastFpsTime) / 1000;
    if (elapsed >= 1) {
      const actualFps = Math.round(frameCount / elapsed);
      set({ actualFps });
      frameCount = 0;
      lastFpsTime = now;
    }
  }, 500);
}

function stopFpsTimer() {
  if (fpsTimer) {
    clearInterval(fpsTimer);
    fpsTimer = null;
  }
  frameCount = 0;
  lastFpsTime = 0;
}

export const useStreamStore = create<StreamState>()(
  persist(
    (set, get) => ({
      isStreaming: false,
      fps: 5,
      actualFps: 0,
      startTime: null,
      elapsedTime: "00:00",

      setFps: async (fps) => {
        set({ fps });
        if (get().isStreaming) {
          await get().stop();
          await get().start();
        }
      },

      start: async (captureAreaOverride) => {
        const { fps } = get();
        const mode = usePreferencesStore.getState().mode;

        let targetType: string;
        let targetIndex: number;
        let targetHwnd: number | null = null;

        if (mode === "screen") {
          const { selectedIndex } = useScreenStore.getState();
          if (selectedIndex === null) return;
          targetType = "screen";
          targetIndex = selectedIndex;
        } else {
          const { selectedIndex, windows } = useWindowStore.getState();
          if (selectedIndex === null) return;
          const win = windows[selectedIndex];
          if (!win?.hwnd) return;
          targetType = "window";
          targetIndex = selectedIndex;
          targetHwnd = win.hwnd;
        }

        try {
          const engine = usePreferencesStore.getState().captureEngine;
          const captureArea = mode === "window"
            ? (captureAreaOverride ?? usePreferencesStore.getState().windowCaptureArea)
            : "frame";
          await startStream(fps, targetType, targetIndex, targetHwnd, engine, captureArea);
          const now = Date.now();
          set({ isStreaming: true, startTime: now, elapsedTime: "00:00", actualFps: 0 });

          elapsedTimer = setInterval(() => {
            const { startTime } = get();
            if (startTime) {
              set({ elapsedTime: formatElapsed(Date.now() - startTime) });
            }
          }, 1000);

          if (usePreferencesStore.getState().showFps) {
            startFpsTimer(set);
          }

          if (!unlistenStream) {
            unlistenStream = await listen<void>("stream-frame", () => {
              frameCount++;

              const url = `http://watchdog.localhost/frame?t=${Date.now()}`;
              const mode = usePreferencesStore.getState().mode;
              useCaptureFrameStore.getState().setStreamFrameUrl(mode, url);
            });
          }
        } catch (e) {
          console.error("Stream start failed:", e);
        }
      },

      stop: async () => {
        try {
          await stopStream();
          set({ isStreaming: false, startTime: null, elapsedTime: "00:00", actualFps: 0 });
          stopFpsTimer();
          if (elapsedTimer) {
            clearInterval(elapsedTimer);
            elapsedTimer = null;
          }
          if (unlistenStream) {
            unlistenStream();
            unlistenStream = null;
          }
        } catch (e) {
          console.error("Stream stop failed:", e);
        }
      },

      refreshStatus: async () => {
        try {
          const status = await getStreamStatus();
          const isStreaming = status.is_streaming;
          const startTime = status.start_time ? Date.now() - status.start_time : null;
          set({
            isStreaming,
            fps: status.fps,
            startTime,
            elapsedTime: startTime ? formatElapsed(Date.now() - startTime) : "00:00",
          });

          if (isStreaming && !elapsedTimer) {
            elapsedTimer = setInterval(() => {
              const { startTime } = get();
              if (startTime) {
                set({ elapsedTime: formatElapsed(Date.now() - startTime) });
              }
            }, 1000);

            if (usePreferencesStore.getState().showFps && !fpsTimer) {
              startFpsTimer(set);
            }
          } else if (!isStreaming && elapsedTimer) {
            clearInterval(elapsedTimer);
            elapsedTimer = null;
            stopFpsTimer();
          }
        } catch (e) {
          console.error("Stream status failed:", e);
        }
      },
    }),
    {
      name: "watchdog:stream",
      partialize: (state) => ({
        fps: state.fps,
      }),
    },
  ),
);
