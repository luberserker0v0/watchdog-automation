import { useStreamStore } from "../stores/useStreamStore";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useRuntimeIntegrationStore } from "../../shell/stores/useRuntimeIntegrationStore";
import { refreshOpenPreviewContext } from "../services/previewWindowManager";

export default function StreamButton() {
  const isStreaming = useStreamStore((s) => s.isStreaming);
  const actualFps = useStreamStore((s) => s.actualFps);
  const elapsedTime = useStreamStore((s) => s.elapsedTime);
  const start = useStreamStore((s) => s.start);
  const stop = useStreamStore((s) => s.stop);
  const captureOwner = useStreamStore((s) => s.captureOwner);
  const captureOwnerId = useStreamStore((s) => s.captureOwnerId);
  const isPipelineRunning = useRuntimeIntegrationStore((s) => s.pipelineExecuting);
  const showFps = usePreferencesStore((s) => s.showFps);

  const isDisabled = isPipelineRunning || captureOwner === "pipeline";

  const handleToggle = async () => {
    if (isDisabled) return;
    if (isStreaming) {
      await stop();
    } else {
      await start();
    }
    await refreshOpenPreviewContext();
  };

  const dotClass = isStreaming ? "on" : "off";
  const btnLabel = isStreaming ? "連線中" : "連線";

  return (
    <div className="stream-group">
      <button
        className={`stream-toggle ${isStreaming ? "active" : ""} ${isDisabled ? "disabled" : ""}`}
        onClick={() => void handleToggle()}
        disabled={isDisabled}
      >
        <span className={`stream-dot ${dotClass}`} />
        {btnLabel}
      </button>
      {showFps && (
        <span className="stream-fps-display">{actualFps} FPS</span>
      )}
      {isStreaming && (
        <span className="stream-duration">{elapsedTime}</span>
      )}
      {captureOwner === "pipeline" && (
        <span className="stream-duration" title={captureOwnerId ?? undefined}>Pipeline controlled</span>
      )}
    </div>
  );
}
