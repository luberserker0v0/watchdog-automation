import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { usePreferencesStore, type CaptureMode } from "../../core/stores/usePreferencesStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useWindowStore } from "../stores/useWindowStore";
import { useStreamStore } from "../stores/useStreamStore";
import { getWindowDetails, type WindowDetailInfo } from "../../../api/window";
import type { PipelineTarget } from "../../../api/pipeline";
import type { StreamStoppedPayload } from "../../../api/stream";
import ModeSelector from "../components/ModeSelector";
import DesktopFilter from "../components/DesktopFilter";
import WindowStateFilter from "../components/WindowStateFilter";
import TargetSelect from "../components/TargetSelect";
import StreamButton from "../components/StreamButton";
import CaptureButton from "../components/CaptureButton";
import ImageLayout from "../components/ImageLayout";
import "../styles/capture.css";

interface UpdateCapturePayload {
  type: "screen" | "window";
  action: "streaming" | "capture";
  target: PipelineTarget;
  hwnd: number | null;
}

function matchWindowByTarget(
  windows: { hwnd?: number; title: string; app_name: string }[],
  target: PipelineTarget,
  hwnd: number | null,
): number {
  if (hwnd) {
    const idx = windows.findIndex((w) => w.hwnd === hwnd);
    if (idx >= 0) return idx;
  }
  if (target.type === "window") {
    const idx = windows.findIndex((w) => {
      if (target.title && !new RegExp(target.title).test(w.title)) return false;
      if (target.process && !new RegExp(target.process).test(w.app_name)) return false;
      return true;
    });
    if (idx >= 0) return idx;
  }
  return -1;
}

async function matchWindowByClass(
  windows: { hwnd?: number; title: string; app_name: string }[],
  target: PipelineTarget,
): Promise<number> {
  if (target.type !== "window" || !target.class) return -1;
  try {
    const details: WindowDetailInfo[] = await getWindowDetails();
    return windows.findIndex((w) => {
      const detail = details.find((d) => d.hwnd === w.hwnd);
      if (!detail) return false;
      return new RegExp(target.class!).test(detail.class_name);
    });
  } catch {
    return -1;
  }
}

export default function CaptureView() {
  const mode = usePreferencesStore((s) => s.mode);
  const fetchScreens = useScreenStore((s) => s.fetchScreens);
  const fetchWindows = useWindowStore((s) => s.fetchWindows);

  useEffect(() => {
    if (mode === "screen") {
      fetchScreens();
    } else {
      fetchWindows();
    }
  }, [mode, fetchScreens, fetchWindows]);

  useEffect(() => {
    const unlisten = listen<UpdateCapturePayload>("update_capture", async (event) => {
      const { type, action, target, hwnd } = event.payload;

      // 1. Switch mode
      if (type !== usePreferencesStore.getState().mode) {
        usePreferencesStore.getState().setMode(type as CaptureMode);
      }

      // 2. Select matching target
      if (type === "window") {
        await useWindowStore.getState().fetchWindows();
        const windows = useWindowStore.getState().windows;

        let matchedIndex = matchWindowByTarget(windows, target, hwnd);

        // Fallback: match by class name via getWindowDetails
        if (matchedIndex < 0 && target.type === "window" && target.class) {
          matchedIndex = await matchWindowByClass(windows, target);
        }

        if (matchedIndex >= 0) {
          useWindowStore.getState().setSelectedIndex(matchedIndex);
        }
      } else {
        const index = target.type === "screen" ? (target.index ?? 0) : 0;
        useScreenStore.getState().setSelectedIndex(index);
      }

      // 3. Start streaming
      if (action === "streaming") {
        const captureArea = target.type === "window" ? target.capture_area : undefined;
        useStreamStore.getState().start(captureArea);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const unlisten = listen<StreamStoppedPayload>("stream-stopped", (event) => {
      useStreamStore.getState().handleBackendStopped(event.payload.target_type);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <div className="capture-view">
      <div className={`control-section ${mode}-mode`}>
        <ModeSelector />
        {mode === "window" && <DesktopFilter />}
        {mode === "window" && <WindowStateFilter />}
        <TargetSelect />
        <div className="control-divider" />
        <div className="capture-actions">
          <StreamButton />
          <CaptureButton />
        </div>
      </div>
      <ImageLayout />
    </div>
  );
}
