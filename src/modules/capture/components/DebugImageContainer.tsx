import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useUiStore } from "../../core/stores/useUiStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import { useOverlayStore } from "../stores/useOverlayStore";
import { useOverlayInteraction } from "../hooks/useOverlayInteraction";
import ExpandButton from "./ExpandButton";

interface DebugImageContainerProps {
  children?: React.ReactNode;
}

export default function DebugImageContainer({ children }: DebugImageContainerProps) {
  const mode = usePreferencesStore((s) => s.mode);
  const previewOpen = useUiStore((s) => s.previewOpen);
  const previewUrl = useCaptureFrameStore((s) => s.frames[mode].previewUrl);
  const visiblePreviewUrl = previewOpen ? null : previewUrl;

  const setCursor = useOverlayStore((s) => s.setCursor);
  const clearCursor = useOverlayStore((s) => s.clearCursor);

  const {
    imgRef,
    containerRef,
    handleMouseMove,
    handleMouseDown,
    handleMouseUp,
    handleMouseLeave,
    renderDragRect,
    renderSelectionRect,
    renderRoiRect,
    renderResultRoiRect,
    renderSidePanel,
  } = useOverlayInteraction({
    onCursorMove: (x, y) => setCursor(x, y),
    onCursorLeave: () => clearCursor(),
  });

  return (
    <div className="debug-image-wrapper">
      <div className="debug-image-layout">
        {renderSidePanel(!!visiblePreviewUrl)}
        <div
          className="debug-image-container"
          ref={containerRef}
          onMouseMove={handleMouseMove}
          onMouseDown={handleMouseDown}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseLeave}
          onDragStart={(e) => e.preventDefault()}
        >
          {visiblePreviewUrl ? (
            <>
              <img
                ref={imgRef}
                src={visiblePreviewUrl}
                alt="截圖預覽"
                draggable={false}
                onDragStart={(e) => e.preventDefault()}
              />
              {renderDragRect()}
              {renderSelectionRect()}
              {renderRoiRect()}
              {renderResultRoiRect()}
            </>
          ) : (
            <div className="preview-placeholder">{previewOpen ? "預覽已展開" : "截圖預覽區域"}</div>
          )}
        </div>
        <div className="debug-side-panel right">
          {visiblePreviewUrl && <ExpandButton />}
        </div>
      </div>
      {children}
    </div>
  );
}
