import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useWindowStore } from "../stores/useWindowStore";
import { useStreamStore } from "../stores/useStreamStore";

export default function CaptureButton() {
  const mode = usePreferencesStore((s) => s.mode);
  const screenCapture = useScreenStore((s) => s.capture);
  const windowCapture = useWindowStore((s) => s.capture);
  const isStreaming = useStreamStore((s) => s.isStreaming);

  const handleCapture = () => {
    if (isStreaming) return;
    if (mode === "screen") {
      screenCapture();
    } else {
      windowCapture();
    }
  };

  return (
    <div className="capture-button">
      <button
        onClick={handleCapture}
        disabled={isStreaming}
        title={isStreaming ? "請先停止串流" : "截圖偵錯"}
      >
        截圖偵錯
      </button>
    </div>
  );
}
