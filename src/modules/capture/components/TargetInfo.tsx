import type { CaptureMode } from "../../core/stores/usePreferencesStore";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useWindowStore } from "../stores/useWindowStore";

const ENGINE_LABELS: Record<string, string> = { gdi: "GDI", dxgi: "DXGI", graphics: "Graphics" };

interface TargetInfoProps {
  targetName?: string;
  mode?: CaptureMode;
  engine?: string;
  statusClass?: "idle" | "streaming" | "captured";
  statusLabel?: string;
}

function TargetInfoView({ targetName, mode, engine, statusClass, statusLabel }: Required<TargetInfoProps>) {
  return (
    <div className="target-info">
      <div className="target-info-left">
        <span className="target-info-icon">{mode === "screen" ? "🖥" : "🪟"}</span>
        <span className="target-info-name">{targetName}</span>
      </div>
      <div className="target-info-right">
        <span className={`target-info-status ${statusClass}`}>{statusLabel}</span>
        {statusClass === "streaming" && <span className="target-info-engine">{ENGINE_LABELS[engine] || engine}</span>}
      </div>
    </div>
  );
}

function useStatus(mode: CaptureMode) {
  const isStreaming = useStreamStore((state) => state.isStreaming);
  const hasCaptured = useCaptureFrameStore((state) => state.frames[mode].captureBuf !== null);
  return isStreaming
    ? { statusClass: "streaming" as const, statusLabel: "串流中" }
    : hasCaptured
      ? { statusClass: "captured" as const, statusLabel: "已截圖" }
      : { statusClass: "idle" as const, statusLabel: "閒置" };
}

function ScreenTargetInfo({ engine }: { engine: string }) {
  const selectedIndex = useScreenStore((state) => state.selectedIndex);
  const screens = useScreenStore((state) => state.screens);
  const status = useStatus("screen");
  const screen = selectedIndex === null ? undefined : screens[selectedIndex];
  const targetName = screen
    ? `Screen ${screen.index} - ${screen.width}×${screen.height}${screen.is_primary ? " (主螢幕)" : ""}`
    : "未選取";
  return <TargetInfoView targetName={targetName} mode="screen" engine={engine} {...status} />;
}

function WindowTargetInfo({ engine }: { engine: string }) {
  const selectedIndex = useWindowStore((state) => state.selectedIndex);
  const windows = useWindowStore((state) => state.windows);
  const status = useStatus("window");
  const windowInfo = selectedIndex === null ? undefined : windows[selectedIndex];
  const targetName = windowInfo
    ? `${windowInfo.app_name || windowInfo.title || "未知"} - ${windowInfo.width}×${windowInfo.height}`
    : "未選取";
  return <TargetInfoView targetName={targetName} mode="window" engine={engine} {...status} />;
}

function ConnectedTargetInfo() {
  const mode = usePreferencesStore((state) => state.mode);
  const engine = usePreferencesStore((state) => state.captureEngine);
  return mode === "screen" ? <ScreenTargetInfo engine={engine} /> : <WindowTargetInfo engine={engine} />;
}

export default function TargetInfo(props: TargetInfoProps) {
  if (props.targetName && props.mode && props.engine && props.statusClass && props.statusLabel) {
    return <TargetInfoView {...props as Required<TargetInfoProps>} />;
  }
  return <ConnectedTargetInfo />;
}
