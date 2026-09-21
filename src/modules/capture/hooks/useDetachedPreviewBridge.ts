import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getStreamStatus } from "../../../api/stream";
import type { StreamStoppedPayload } from "../../../api/stream";
import type { CaptureArea, CaptureMode } from "../../shell/stores/usePreferencesStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import { useOverlayStore } from "../stores/useOverlayStore";
import type { PipelineTarget } from "../../../api/pipeline";

export interface TargetInfoPayload {
  targetName: string;
  mode: CaptureMode;
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

export function useDetachedPreviewBridge() {
  const [streaming, setStreaming] = useState(false);
  const [captureMode, setCaptureMode] = useState<CaptureMode>("screen");
  const [targetInfo, setTargetInfo] = useState<TargetInfoPayload | null>(null);

  useEffect(() => {
    let disposed = false;
    let frameTimer: ReturnType<typeof setInterval> | null = null;
    let sessionId = 0;
    const cleanups: Array<() => void> = [];
    const frameStore = useCaptureFrameStore.getState();
    const resetOverlay = () => {
      const overlay = useOverlayStore.getState();
      overlay.clearAll();
      overlay.clearCursor();
      overlay.setImageDimensions(null);
    };

    const setup = async () => {
      const stopFrameTimer = () => {
        if (!frameTimer) return;
        clearInterval(frameTimer);
        frameTimer = null;
      };

      const syncStream = async () => {
        try {
          const status = await getStreamStatus();
          if (disposed) return;
          if (!status.is_streaming) {
            sessionId = 0;
            stopFrameTimer();
            setStreaming(false);
            resetOverlay();
            return;
          }

          if (sessionId !== status.session_id) resetOverlay();
          sessionId = status.session_id;
          const mode = status.target_type as CaptureMode;
          setStreaming(true);
          setCaptureMode(mode);
          stopFrameTimer();
          frameStore.setStreamFrameUrl(mode, `http://watchdog.localhost/frame?t=${Date.now()}`);
          frameTimer = setInterval(() => {
            frameStore.setStreamFrameUrl(mode, `http://watchdog.localhost/frame?t=${Date.now()}`);
          }, 100);
        } catch {
          // A detached still-image preview does not require an active stream.
        }
      };

      const unlistenStopped = await listen<StreamStoppedPayload>("stream-stopped", (event) => {
        if (disposed) return;
        if (sessionId && event.payload.session_id !== sessionId) return;
        stopFrameTimer();
        sessionId = 0;
        setStreaming(false);
        frameStore.clearFrame(event.payload.target_type);
        resetOverlay();
      });
      if (disposed) { unlistenStopped(); return; } else cleanups.push(unlistenStopped);

      const unlistenTarget = await listen<TargetInfoPayload>("preview-target-info", (event) => {
        if (disposed) return;
        resetOverlay();
        setTargetInfo(event.payload);
        setCaptureMode(event.payload.mode);
        void syncStream();
      });
      if (disposed) { unlistenTarget(); return; } else cleanups.push(unlistenTarget);

      const unlistenImage = await listen<{ bytes: number[]; mode: CaptureMode }>("preview-image", (event) => {
        const { bytes, mode } = event.payload;
        const url = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: "image/png" }));
        frameStore.setCapturedFrame(mode, url, bytes);
        setCaptureMode(mode);
        resetOverlay();
      });
      if (disposed) { unlistenImage(); return; } else cleanups.push(unlistenImage);

      await syncStream();
      await getCurrentWebviewWindow().emit("preview-ready");
    };

    void setup();
    return () => {
      disposed = true;
      if (frameTimer) clearInterval(frameTimer);
      cleanups.forEach((cleanup) => cleanup());
      frameStore.clearFrame("screen");
      frameStore.clearFrame("window");
      void getCurrentWebviewWindow().emit("preview-closed");
    };
  }, []);

  return { streaming, captureMode, targetInfo };
}
