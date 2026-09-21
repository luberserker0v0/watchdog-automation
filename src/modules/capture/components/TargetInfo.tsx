import type { CaptureMode } from "../../shell/stores/usePreferencesStore";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import { useOverlayStore } from "../stores/useOverlayStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useWindowStore } from "../stores/useWindowStore";
import { usePipelineStore } from "../../pipeline/runtime/stores/usePipelineStore";

const ENGINE_LABELS: Record<string, string> = { gdi: "GDI", dxgi: "DXGI", graphics: "Graphics" };

interface TargetInfoProps {
  targetName?: string;
  mode?: CaptureMode;
  engine?: string;
  statusClass?: "idle" | "streaming" | "captured";
  statusLabel?: string;
  frameSize?: [number, number] | null;
  clientSize?: [number, number] | null;
  expectedSize?: [number, number] | null;
}

interface TargetInfoViewProps {
  targetName: string;
  mode: CaptureMode;
  engine: string;
  statusClass: "idle" | "streaming" | "captured";
  statusLabel: string;
  frameSize?: [number, number] | null;
  clientSize?: [number, number] | null;
  expectedSize?: [number, number] | null;
}

function TargetInfoView({ targetName, mode, engine, statusClass, statusLabel, frameSize, clientSize, expectedSize }: TargetInfoViewProps) {
  const expectedMatches = expectedSize && clientSize
    ? Math.abs(expectedSize[0] - clientSize[0]) <= 2 && Math.abs(expectedSize[1] - clientSize[1]) <= 2
    : null;
  return (
    <div className="target-info">
      <div className="target-info-left">
        <span className="target-info-icon">{mode === "screen" ? "🖥" : "🪟"}</span>
        <span className="target-info-name">{targetName}</span>
      </div>
      <div className="target-info-right">
        {mode === "window" && clientSize && (
          <span className="target-info-size" title="Current window client/content size">
            Content {clientSize[0]}×{clientSize[1]}
          </span>
        )}
        {frameSize && (
          <span className="target-info-size" title="Actual encoded capture image size">
            Capture {frameSize[0]}×{frameSize[1]}
          </span>
        )}
        {expectedSize && (
          <span
            className={`target-info-size expected${expectedMatches === null ? "" : expectedMatches ? " matches" : " mismatch"}`}
            title={expectedMatches === null
              ? "Pipeline expected client/content size"
              : expectedMatches ? "Current content size matches the pipeline requirement" : "Current content size does not match the pipeline requirement"}
          >
            Expected {expectedSize[0]}×{expectedSize[1]}{expectedMatches === null ? "" : expectedMatches ? " ✓" : " !"}
          </span>
        )}
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

function ScreenTargetInfo({ engine, frameSize }: { engine: string; frameSize: [number, number] | null }) {
  const selectedIndex = useScreenStore((state) => state.selectedIndex);
  const screens = useScreenStore((state) => state.screens);
  const status = useStatus("screen");
  const screen = selectedIndex === null ? undefined : screens[selectedIndex];
  const targetName = screen
    ? `Screen ${screen.index} - ${screen.width}×${screen.height}${screen.is_primary ? " (主螢幕)" : ""}`
    : "未選取";
  return <TargetInfoView targetName={targetName} mode="screen" engine={engine} frameSize={frameSize} {...status} />;
}

function WindowTargetInfo({ engine, frameSize, clientSize, expectedSize }: { engine: string; frameSize: [number, number] | null; clientSize: [number, number] | null; expectedSize: [number, number] | null }) {
  const selectedIndex = useWindowStore((state) => state.selectedIndex);
  const windows = useWindowStore((state) => state.windows);
  const status = useStatus("window");
  const windowInfo = selectedIndex === null ? undefined : windows[selectedIndex];
  const targetName = windowInfo
    ? windowInfo.app_name || windowInfo.title || "未知"
    : "未選取";
  return <TargetInfoView targetName={targetName} mode="window" engine={engine} frameSize={frameSize} clientSize={clientSize} expectedSize={expectedSize} {...status} />;
}

function ConnectedTargetInfo({ frameSize, clientSize, expectedSize }: { frameSize: [number, number] | null; clientSize: [number, number] | null; expectedSize: [number, number] | null }) {
  const mode = usePreferencesStore((state) => state.mode);
  const engine = usePreferencesStore((state) => state.captureEngine);
  return mode === "screen"
    ? <ScreenTargetInfo engine={engine} frameSize={frameSize} />
    : <WindowTargetInfo engine={engine} frameSize={frameSize} clientSize={clientSize} expectedSize={expectedSize} />;
}

export default function TargetInfo(props: TargetInfoProps) {
  const currentMode = usePreferencesStore((state) => state.mode);
  const windowCaptureArea = usePreferencesStore((state) => state.windowCaptureArea);
  const imageDimensions = useOverlayStore((state) => state.imageDimensions);
  const streamFrameSize = useStreamStore((state) => state.frameSize);
  const streamClientSize = useStreamStore((state) => state.clientSize);
  const isStreaming = useStreamStore((state) => state.isStreaming);
  const pipelineExpectedSize = usePipelineStore((state) => {
    if (!state.activeId) return null;
    const target = state.pipelinesById[state.activeId]?.graph?.target;
    return target?.type === "window" ? target.window_size ?? null : null;
  });
  const resizeIssue = usePipelineStore((state) => state.resizeIssue);
  const effectiveMode = props.mode ?? currentMode;
  const hasFrame = useCaptureFrameStore((state) => state.frames[effectiveMode].previewUrl !== null);
  const fallbackFrameSize: [number, number] | null = hasFrame && imageDimensions
    ? [imageDimensions.width, imageDimensions.height]
    : null;
  const frameSize = props.frameSize ?? (isStreaming ? streamFrameSize : fallbackFrameSize);
  const clientSize = props.clientSize ?? streamClientSize ?? (isStreaming
    ? null
    : resizeIssue?.before ?? (
      effectiveMode === "window" && windowCaptureArea === "client" ? fallbackFrameSize : null
    ));
  const expectedSize = props.expectedSize ?? pipelineExpectedSize;
  if (props.targetName && props.mode && props.engine && props.statusClass && props.statusLabel) {
    return <TargetInfoView {...props as TargetInfoViewProps} frameSize={frameSize} clientSize={clientSize} expectedSize={expectedSize} />;
  }
  return <ConnectedTargetInfo frameSize={frameSize} clientSize={clientSize} expectedSize={expectedSize} />;
}
