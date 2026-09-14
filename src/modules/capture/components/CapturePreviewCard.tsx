import { useState } from "react";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useUiStore } from "../../core/stores/useUiStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import TargetInfo from "./TargetInfo";
import ExpandButton from "./ExpandButton";

export default function CapturePreviewCard() {
  const mode = usePreferencesStore((state) => state.mode);
  const previewOpen = useUiStore((state) => state.previewOpen);
  const previewUrl = useCaptureFrameStore((state) => state.frames[mode].previewUrl);
  const [frameSize, setFrameSize] = useState<[number, number] | null>(null);

  return (
    <section className="capture-preview-card" aria-label="Latest capture">
      <TargetInfo frameSize={frameSize} />
      <div className="capture-thumbnail">
        {previewUrl ? (
          <img
            src={previewUrl}
            alt="Latest capture"
            draggable={false}
            onLoad={(event) => {
              const next: [number, number] = [
                event.currentTarget.naturalWidth,
                event.currentTarget.naturalHeight,
              ];
              setFrameSize((current) => current?.[0] === next[0] && current[1] === next[1] ? current : next);
            }}
          />
        ) : (
          <div className="capture-thumbnail-empty">
            <span aria-hidden="true">▧</span>
            <strong>No capture yet</strong>
            <small>Select a target, then capture once or start streaming.</small>
          </div>
        )}
      </div>
      <footer className="capture-preview-footer">
        <span>{previewOpen ? "Preview workspace is open" : "Open Preview for ROI and debug tools"}</span>
        <ExpandButton label={previewOpen ? "Focus Preview" : "Open Preview"} />
      </footer>
    </section>
  );
}
