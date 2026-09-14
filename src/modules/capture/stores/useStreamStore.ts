import { create } from "zustand";
import { persist } from "zustand/middleware";
import { listen } from "@tauri-apps/api/event";
import {
  startStream,
  stopStream,
  getStreamStatus,
  type StreamStoppedPayload,
  type StreamFramePayload,
} from "../../../api/stream";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useScreenStore } from "./useScreenStore";
import { useWindowStore } from "./useWindowStore";
import { useCaptureFrameStore } from "./useCaptureFrameStore";
import type { CaptureArea, CaptureMode } from "../../core/stores/usePreferencesStore";

interface StreamState {
  isStreaming: boolean;
  fps: number;
  actualFps: number;
  startTime: number | null;
  elapsedTime: string;
  captureOwner: string;
  captureOwnerId: string | null;
  sessionId: number;
  frameSize: [number, number] | null;
  clientSize: [number, number] | null;
  setCaptureSizes: (frameSize: [number, number] | null, clientSize: [number, number] | null) => void;
  setFps: (fps: number) => void;
  start: (captureAreaOverride?: CaptureArea) => Promise<void>;
  stop: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  handleBackendStopped: (payload: StreamStoppedPayload | CaptureMode) => void;
}

let unlistenStream: (() => void) | null = null;
let unlistenStopped: (() => void) | null = null;
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

async function ensureStreamListeners(get: () => StreamState) {
  if (!unlistenStopped) {
    unlistenStopped = await listen<StreamStoppedPayload>("stream-stopped", (event) => {
      get().handleBackendStopped(event.payload);
    });
  }
  if (!unlistenStream) {
    unlistenStream = await listen<StreamFramePayload>("stream-frame", (event) => {
      if (get().sessionId && event.payload?.session_id !== get().sessionId) return;
      frameCount++;
      const mode = event.payload?.target_type ?? usePreferencesStore.getState().mode;
      const frameSize: [number, number] | null = event.payload?.frame_width && event.payload?.frame_height
        ? [event.payload.frame_width, event.payload.frame_height]
        : null;
      const clientSize: [number, number] | null = event.payload?.client_width && event.payload?.client_height
        ? [event.payload.client_width, event.payload.client_height]
        : null;
      get().setCaptureSizes(frameSize, clientSize);
      useCaptureFrameStore.getState().setStreamFrameUrl(
        mode,
        "http://watchdog.localhost/frame?t=" + Date.now(),
      );
    });
  }
}

export const useStreamStore = create<StreamState>()(
  persist(
    (set, get) => ({
      isStreaming: false,
      fps: 5,
      actualFps: 0,
      startTime: null,
      elapsedTime: "00:00",
      captureOwner: "none",
      captureOwnerId: null,
      sessionId: 0,
      frameSize: null,
      clientSize: null,

      setCaptureSizes: (frameSize, clientSize) => set({ frameSize, clientSize }),

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

        await ensureStreamListeners(get);

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
          set({ isStreaming: true, startTime: now, elapsedTime: "00:00", actualFps: 0, captureOwner: "manual", captureOwnerId: null, frameSize: null, clientSize: null });

          elapsedTimer = setInterval(() => {
            const { startTime } = get();
            if (startTime) {
              set({ elapsedTime: formatElapsed(Date.now() - startTime) });
            }
          }, 1000);

          if (usePreferencesStore.getState().showFps) {
            startFpsTimer(set);
          }

        } catch (e) {
          console.error("Stream start failed:", e);
        }
      },

      stop: async () => {
        try {
          await stopStream();
          set({ isStreaming: false, startTime: null, elapsedTime: "00:00", actualFps: 0, captureOwner: "none", captureOwnerId: null, sessionId: 0, frameSize: null, clientSize: null });
          stopFpsTimer();
          if (elapsedTimer) {
            clearInterval(elapsedTimer);
            elapsedTimer = null;
          }
          if (unlistenStream) {
            unlistenStream();
            unlistenStream = null;
          }
          if (unlistenStopped) {
            unlistenStopped();
            unlistenStopped = null;
          }
          useCaptureFrameStore.getState().clearFrame(usePreferencesStore.getState().mode);
        } catch (e) {
          console.error("Stream stop failed:", e);
        }
      },

      handleBackendStopped: (payload) => {
        const targetType = typeof payload === "string" ? payload : payload.target_type;
        if (typeof payload !== "string" && get().sessionId && payload.session_id !== get().sessionId) return;
        set({ isStreaming: false, startTime: null, elapsedTime: "00:00", actualFps: 0, captureOwner: "none", captureOwnerId: null, sessionId: 0, frameSize: null, clientSize: null });
        stopFpsTimer();
        if (elapsedTimer) {
          clearInterval(elapsedTimer);
          elapsedTimer = null;
        }
        if (unlistenStream) {
          unlistenStream();
          unlistenStream = null;
        }
        if (unlistenStopped) {
          unlistenStopped();
          unlistenStopped = null;
        }
        useCaptureFrameStore.getState().clearFrame(targetType);
      },

      refreshStatus: async () => {
        try {
          const status = await getStreamStatus();
          const isStreaming = status.is_streaming;
          const startTime = status.start_time ? Date.now() - status.start_time : null;
          if (isStreaming && (status.target_type === "screen" || status.target_type === "window")) {
            usePreferencesStore.getState().setMode(status.target_type);
            if (status.target_type === "window") {
              usePreferencesStore.getState().setWindowCaptureArea(status.capture_area);
            }
            await ensureStreamListeners(get);
          }
          set({
            isStreaming,
            fps: status.fps,
            startTime,
            elapsedTime: startTime ? formatElapsed(Date.now() - startTime) : "00:00",
            captureOwner: status.owner,
            captureOwnerId: status.owner_id,
            sessionId: status.session_id,
            frameSize: status.frame_width && status.frame_height ? [status.frame_width, status.frame_height] : null,
            clientSize: status.client_width && status.client_height ? [status.client_width, status.client_height] : null,
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
          if (!isStreaming) {
            useCaptureFrameStore.getState().clearFrame("screen");
            useCaptureFrameStore.getState().clearFrame("window");
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
