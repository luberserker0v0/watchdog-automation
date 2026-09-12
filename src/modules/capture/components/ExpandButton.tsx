import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen } from "@tauri-apps/api/event";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useUiStore } from "../../core/stores/useUiStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useWindowStore } from "../stores/useWindowStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";

interface TargetInfoPayload {
  targetName: string;
  mode: "screen" | "window";
  engine: string;
  statusClass: "idle" | "streaming" | "captured";
  statusLabel: string;
}

function computeTargetInfo(): TargetInfoPayload {
  const mode = usePreferencesStore.getState().mode;
  const engine = usePreferencesStore.getState().captureEngine;
  const isStreaming = useStreamStore.getState().isStreaming;

  let targetName = "未選取";
  let statusClass: "idle" | "streaming" | "captured" = "idle";
  let statusLabel = "閒置";

  if (isStreaming) {
    statusClass = "streaming";
    statusLabel = "串流中";
  }

  if (mode === "screen") {
    const { selectedIndex, screens } = useScreenStore.getState();
    const captureBuf = useCaptureFrameStore.getState().frames.screen.captureBuf;
    if (!isStreaming && captureBuf !== null) {
      statusClass = "captured";
      statusLabel = "已截圖";
    }
    if (selectedIndex !== null) {
      const s = screens[selectedIndex];
      if (s) {
        targetName = `Screen ${s.index} - ${s.width}×${s.height}`;
        if (s.is_primary) targetName += " (主螢幕)";
      }
    }
  } else {
    const { selectedIndex, windows } = useWindowStore.getState();
    const captureBuf = useCaptureFrameStore.getState().frames.window.captureBuf;
    if (!isStreaming && captureBuf !== null) {
      statusClass = "captured";
      statusLabel = "已截圖";
    }
    if (selectedIndex !== null) {
      const w = windows[selectedIndex];
      if (w) {
        const label = w.app_name || w.title || "未知";
        targetName = `${label} - ${w.width}×${w.height}`;
      }
    }
  }

  return { targetName, mode, engine, statusClass, statusLabel };
}

export default function ExpandButton() {
  const mode = usePreferencesStore((s) => s.mode);
  const previewOpen = useUiStore((s) => s.previewOpen);
  const setPreviewOpen = useUiStore((s) => s.setPreviewOpen);
  const isStreaming = useStreamStore((s) => s.isStreaming);

  const handleExpand = async () => {
    if (previewOpen) return;

    const label = `preview-${Date.now()}`;
    setPreviewOpen(true);

    const win = new WebviewWindow(label, {
      url: "index.html#preview",
      title: mode === "screen" ? "螢幕預覽" : "視窗預覽",
      width: 1200,
      height: 800,
      minWidth: 600,
      minHeight: 400,
      center: true,
      decorations: false,
    });

    win.once("tauri://destroyed", () => {
      setPreviewOpen(false);
    });

    win.once("tauri://error", (e) => {
      console.error("Preview window creation failed:", e);
      setPreviewOpen(false);
    });

    const targetInfo = computeTargetInfo();

    if (!isStreaming) {
      const buf = useCaptureFrameStore.getState().frames[mode].captureBuf;
      if (!buf) return;

      const unlisten = await listen("preview-ready", () => {
        win.emit("preview-image", { bytes: buf, mode });
        win.emit("preview-target-info", targetInfo);
        unlisten();
      });
    } else {
      const unlisten = await listen("preview-ready", () => {
        win.emit("preview-target-info", targetInfo);
        unlisten();
      });
    }
  };

  return (
    <button
      className="debug-expand-btn"
      onClick={handleExpand}
      disabled={previewOpen}
      title="展開預覽"
    >
      ⛶
    </button>
  );
}
