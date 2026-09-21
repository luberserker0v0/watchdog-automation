import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useStreamStore } from "../stores/useStreamStore";

export default function ModeSelector() {
  const mode = usePreferencesStore((s) => s.mode);
  const setMode = usePreferencesStore((s) => s.setMode);
  const pipelineControlled = useStreamStore((state) => state.captureOwner === "pipeline");

  return (
    <div className="mode-selector">
      <button
        className={mode === "screen" ? "active" : ""}
        disabled={pipelineControlled}
        onClick={() => setMode("screen")}
      >
        螢幕截圖
      </button>
      <button
        className={mode === "window" ? "active" : ""}
        disabled={pipelineControlled}
        onClick={() => setMode("window")}
      >
        視窗截圖
      </button>
    </div>
  );
}
