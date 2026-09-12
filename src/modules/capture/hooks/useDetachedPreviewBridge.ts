import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getStreamStatus } from "../../../api/stream";
import type { StreamStoppedPayload } from "../../../api/stream";
import type { CaptureMode } from "../../core/stores/usePreferencesStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import { useOverlayStore } from "../stores/useOverlayStore";

export interface TargetInfoPayload {
  targetName: string;
  mode: CaptureMode;
  engine: string;
  statusClass: "idle" | "streaming" | "captured";
  statusLabel: string;
}

export function useDetachedPreviewBridge() {
  const [streaming, setStreaming] = useState(false);
  const [captureMode, setCaptureMode] = useState<CaptureMode>("screen");
  const [targetInfo, setTargetInfo] = useState<TargetInfoPayload | null>(null);

  useEffect(() => {
    let disposed = false;
    let frameTimer: ReturnType<typeof setInterval> | null = null;
    const cleanups: Array<() => void> = [];
    const frameStore = useCaptureFrameStore.getState();

    const setup = async () => {
      const unlistenStopped = await listen<StreamStoppedPayload>("stream-stopped", (event) => {
        if (disposed) return;
        if (frameTimer) {
          clearInterval(frameTimer);
          frameTimer = null;
        }
        setStreaming(false);
        frameStore.clearFrame(event.payload.target_type);
      });
      if (disposed) { unlistenStopped(); return; } else cleanups.push(unlistenStopped);

      const unlistenTarget = await listen<TargetInfoPayload>("preview-target-info", (event) => {
        if (!disposed) setTargetInfo(event.payload);
      });
      if (disposed) { unlistenTarget(); return; } else cleanups.push(unlistenTarget);

      try {
        const status = await getStreamStatus();
        if (status.is_streaming) {
          const mode = status.target_type as CaptureMode;
          if (!disposed) {
            setStreaming(true);
            setCaptureMode(mode);
            frameStore.setStreamFrameUrl(mode, `http://watchdog.localhost/frame?t=${Date.now()}`);
            frameTimer = setInterval(() => {
              frameStore.setStreamFrameUrl(mode, `http://watchdog.localhost/frame?t=${Date.now()}`);
            }, 100);
          }
          await getCurrentWebviewWindow().emit("preview-ready");
          return;
        }
      } catch {
        // A detached still-image preview does not require an active stream.
      }

      const unlistenImage = await listen<{ bytes: number[]; mode: CaptureMode }>("preview-image", (event) => {
        const { bytes, mode } = event.payload;
        const url = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: "image/png" }));
        frameStore.setCapturedFrame(mode, url, bytes);
        setCaptureMode(mode);
        useOverlayStore.getState().clearAll();
        useOverlayStore.getState().clearCursor();
      });
      if (disposed) { unlistenImage(); return; } else cleanups.push(unlistenImage);
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
