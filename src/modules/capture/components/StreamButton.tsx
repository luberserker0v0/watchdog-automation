import { useStreamStore } from "../stores/useStreamStore";
import { usePipelineStore } from "../../pipeline/stores/usePipelineStore";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";

export default function StreamButton() {
  const isStreaming = useStreamStore((s) => s.isStreaming);
  const actualFps = useStreamStore((s) => s.actualFps);
  const elapsedTime = useStreamStore((s) => s.elapsedTime);
  const start = useStreamStore((s) => s.start);
  const stop = useStreamStore((s) => s.stop);
  const isPipelineRunning = usePipelineStore((s) => s.isExecuting);
  const showFps = usePreferencesStore((s) => s.showFps);

  const isDisabled = isPipelineRunning;

  const handleToggle = () => {
    if (isDisabled) return;
    if (isStreaming) {
      stop();
    } else {
      start();
    }
  };

  const dotClass = isStreaming ? "on" : "off";
  const btnLabel = isStreaming ? "連線中" : "連線";

  return (
    <div className="stream-group">
      <button
        className={`stream-toggle ${isStreaming ? "active" : ""} ${isDisabled ? "disabled" : ""}`}
        onClick={handleToggle}
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
    </div>
  );
}
