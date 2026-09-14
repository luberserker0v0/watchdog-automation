import { usePreferencesStore, type CaptureArea } from "../../core/stores/usePreferencesStore";
import { useStreamStore } from "../stores/useStreamStore";

export default function CaptureAreaSelector() {
  const value = usePreferencesStore((state) => state.windowCaptureArea);
  const setValue = usePreferencesStore((state) => state.setWindowCaptureArea);
  const isStreaming = useStreamStore((state) => state.isStreaming);
  const pipelineControlled = useStreamStore((state) => state.captureOwner === "pipeline");

  return (
    <label className="capture-area-selector">
      <span>Area</span>
      <select
        aria-label="Capture area"
        value={value}
        disabled={isStreaming || pipelineControlled}
        title={pipelineControlled ? "Pipeline controls capture" : isStreaming ? "Stop streaming before changing capture area" : undefined}
        onChange={(event) => setValue(event.target.value as CaptureArea)}
      >
        <option value="frame">Window frame</option>
        <option value="client">Window client</option>
      </select>
    </label>
  );
}
