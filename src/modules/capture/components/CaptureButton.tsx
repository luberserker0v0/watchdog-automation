import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useWindowStore } from "../stores/useWindowStore";
import { useStreamStore } from "../stores/useStreamStore";
import { refreshOpenPreviewContext } from "../services/previewWindowManager";

export default function CaptureButton() {
  const mode = usePreferencesStore((s) => s.mode);
  const screenCapture = useScreenStore((s) => s.capture);
  const windowCapture = useWindowStore((s) => s.capture);
  const isStreaming = useStreamStore((s) => s.isStreaming);
  const pipelineControlled = useStreamStore((state) => state.captureOwner === "pipeline");

  const handleCapture = async () => {
    if (isStreaming || pipelineControlled) return;
    if (mode === "screen") {
      await screenCapture();
    } else {
      await windowCapture();
    }
    await refreshOpenPreviewContext();
  };

  return (
    <div className="capture-button">
      <button
        onClick={() => void handleCapture()}
        disabled={isStreaming || pipelineControlled}
        title={pipelineControlled ? "Pipeline controls capture" : isStreaming ? "請先停止串流" : "截圖偵錯"}
      >
        截圖偵錯
      </button>
    </div>
  );
}
