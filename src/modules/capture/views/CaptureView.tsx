import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useUiStore } from "../../core/stores/useUiStore";
import ModeSelector from "../components/ModeSelector";
import DesktopFilter from "../components/DesktopFilter";
import WindowStateFilter from "../components/WindowStateFilter";
import TargetSelect from "../components/TargetSelect";
import StreamButton from "../components/StreamButton";
import CaptureButton from "../components/CaptureButton";
import WindowResizeControl from "../components/WindowResizeControl";
import CaptureAreaSelector from "../components/CaptureAreaSelector";
import CapturePreviewCard from "../components/CapturePreviewCard";
import ExpandButton from "../components/ExpandButton";
import { refreshOpenPreviewContext } from "../services/previewWindowManager";
import "../styles/capture.css";

export default function CaptureView() {
  const mode = usePreferencesStore((s) => s.mode);
  const capturePanelOpen = useUiStore((state) => state.capturePanelOpen);
  const setCapturePanelOpen = useUiStore((state) => state.setCapturePanelOpen);

  if (!capturePanelOpen) {
    return <CaptureRail onExpand={() => setCapturePanelOpen(true)} />;
  }

  return (
    <div className="capture-view">
      <div className={`control-section ${mode}-mode`}>
        <div className="capture-mode-row">
          <ModeSelector />
          <button
            type="button"
            className="capture-collapse-button"
            onClick={() => setCapturePanelOpen(false)}
            title="Collapse capture panel"
            aria-label="Collapse capture panel"
          >
            ›
          </button>
        </div>
        <div className="capture-primary-row">
          <TargetSelect />
          {mode === "window" && <CaptureAreaSelector />}
        </div>
        {mode === "window" && (
          <details className="capture-advanced">
            <summary>Window options</summary>
            <div className="capture-advanced-content">
              <DesktopFilter />
              <WindowStateFilter />
              <WindowResizeControl />
            </div>
          </details>
        )}
        <div className="capture-actions">
          <StreamButton />
          <CaptureButton />
        </div>
      </div>
      <CapturePreviewCard />
    </div>
  );
}

function CaptureRail({ onExpand }: { onExpand: () => void }) {
  const mode = usePreferencesStore((state) => state.mode);
  const isStreaming = useStreamStore((state) => state.isStreaming);
  const captureOwner = useStreamStore((state) => state.captureOwner);
  const start = useStreamStore((state) => state.start);
  const stop = useStreamStore((state) => state.stop);
  const pipelineControlled = captureOwner === "pipeline";
  const toggleStream = async () => {
    if (isStreaming) await stop();
    else await start();
    await refreshOpenPreviewContext();
  };

  return (
    <aside className="capture-rail" aria-label="Capture controls">
      <button type="button" className="capture-rail-button capture-rail-expand" onClick={onExpand} title="Expand capture panel" aria-label="Expand capture panel">‹</button>
      <span className="capture-rail-mode" title={mode === "screen" ? "Screen capture" : "Window capture"} aria-label={mode === "screen" ? "Screen capture" : "Window capture"}>
        {mode === "screen" ? "🖥" : "🪟"}
      </span>
      <span className={`capture-rail-status ${isStreaming ? "streaming" : "idle"}`} title={pipelineControlled ? "Pipeline controls capture" : isStreaming ? "Streaming" : "Idle"} />
      <button
        type="button"
        className={`capture-rail-button ${isStreaming ? "active" : ""}`}
        disabled={pipelineControlled}
        onClick={() => void toggleStream()}
        title={pipelineControlled ? "Pipeline controls capture" : isStreaming ? "Stop streaming" : "Start streaming"}
        aria-label={isStreaming ? "Stop streaming" : "Start streaming"}
      >
        {isStreaming ? "■" : "▶"}
      </button>
      <div className="capture-rail-spacer" />
      <ExpandButton />
    </aside>
  );
}
