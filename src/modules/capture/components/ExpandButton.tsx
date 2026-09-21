import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useUiStore } from "../../shell/stores/useUiStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import { openOrFocusPreview } from "../services/previewWindowManager";

interface ExpandButtonProps {
  label?: string;
}

export default function ExpandButton({ label }: ExpandButtonProps) {
  const mode = usePreferencesStore((s) => s.mode);
  const previewOpen = useUiStore((s) => s.previewOpen);
  const setPreviewOpen = useUiStore((s) => s.setPreviewOpen);
  const isStreaming = useStreamStore((s) => s.isStreaming);
  const hasFrame = useCaptureFrameStore((state) => state.frames[mode].previewUrl !== null);

  const handleExpand = async () => {
    try {
      await openOrFocusPreview();
    } catch (error) {
      console.error("Preview window creation failed:", error);
      setPreviewOpen(false);
    }
  };

  return (
    <button
      className="debug-expand-btn"
      onClick={handleExpand}
      disabled={!isStreaming && !hasFrame}
      title={previewOpen ? "Focus Preview" : "Open Preview"}
    >
      <span aria-hidden="true">⛶</span>{label && <span>{label}</span>}
    </button>
  );
}
