import { useEffect } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { CaptureMode } from "../stores/usePreferencesStore";
import DebugWorkbench from "../../testing/components/DebugWorkbench";
import TargetInfo from "../../capture/components/TargetInfo";
import { useDetachedPreviewBridge } from "../../capture/hooks/useDetachedPreviewBridge";
import { useOverlayInteraction } from "../../capture/hooks/useOverlayInteraction";
import { useCaptureFrameStore } from "../../capture/stores/useCaptureFrameStore";
import { useOverlayStore } from "../../capture/stores/useOverlayStore";
import type { ActionTestContext } from "../../../api/pipeline";

function PreviewFrame({ mode }: { mode: CaptureMode }) {
  const previewUrl = useCaptureFrameStore((state) => state.frames[mode].previewUrl);
  const setCursor = useOverlayStore((state) => state.setCursor);
  const clearCursor = useOverlayStore((state) => state.clearCursor);
  const setImageDimensions = useOverlayStore((state) => state.setImageDimensions);
  const interaction = useOverlayInteraction({ onCursorMove: setCursor, onCursorLeave: clearCursor, resetKey: mode });

  useEffect(() => {
    if (interaction.lastCoords) {
      setImageDimensions({
        width: interaction.lastCoords.naturalW,
        height: interaction.lastCoords.naturalH,
      });
    }
  }, [interaction.lastCoords, setImageDimensions]);

  return (
    <div className="preview-page-image-area">
      {interaction.renderSidePanel(!!previewUrl)}
      <div
        className={`preview-page-image-container interaction-${interaction.interactionMode}${interaction.isPanning ? " is-panning" : ""}`}
        ref={interaction.containerRef}
        onMouseMove={interaction.handleMouseMove}
        onMouseDown={interaction.handleMouseDown}
        onMouseUp={interaction.handleMouseUp}
        onMouseLeave={interaction.handleMouseLeave}
        onDragStart={(event) => event.preventDefault()}
      >
        {previewUrl ? (
          <>
            <img
              ref={interaction.imgRef}
              src={previewUrl}
              alt="預覽"
              draggable={false}
              style={interaction.imageStyle}
              onLoad={(event) => setImageDimensions({
                width: event.currentTarget.naturalWidth,
                height: event.currentTarget.naturalHeight,
              })}
            />
            {interaction.renderDragRect()}
            {interaction.renderSelectionRect()}
            {interaction.renderRoiRect()}
            {interaction.renderResultRoiRect()}
          </>
        ) : <div className="preview-placeholder">載入中...</div>}
      </div>
      <div className="debug-side-panel right" />
    </div>
  );
}

function PreviewWorkbench({
  mode,
  streaming,
  actionContext,
  currentTarget,
}: {
  mode: CaptureMode;
  streaming: boolean;
  actionContext?: ActionTestContext;
  currentTarget?: import("../../../api/pipeline").PipelineTarget;
}) {
  const hasPreview = useCaptureFrameStore((state) => state.frames[mode].previewUrl !== null);
  const captureBuf = useCaptureFrameStore((state) => state.frames[mode].captureBuf);
  const captureLabel = mode === "screen" ? "Screen" : `Window ${actionContext?.type === "window" && actionContext.capture_area === "client" ? "client" : "frame"}`;

  return (
    <aside className="preview-page-tabs" aria-label="Debug workspace">
      <DebugWorkbench
        context={{ captureMode: mode, isStreaming: streaming, captureBuf, hasFrame: hasPreview, currentTarget, actionContext }}
        captureControl={{ value: captureLabel, label: captureLabel, readOnly: true }}
      />
    </aside>
  );
}

export default function PreviewPage() {
  const { streaming, captureMode, targetInfo } = useDetachedPreviewBridge();
  const imageDimensions = useOverlayStore((state) => state.imageDimensions);
  const actionContext: ActionTestContext | undefined = targetInfo?.targetIndex != null && imageDimensions
    ? captureMode === "screen"
      ? {
          type: "screen",
          index: targetInfo.targetIndex,
          frame_width: imageDimensions.width,
          frame_height: imageDimensions.height,
        }
      : {
          type: "window",
          hwnd: targetInfo.targetIndex,
          capture_area: targetInfo.captureArea ?? "frame",
          frame_width: imageDimensions.width,
          frame_height: imageDimensions.height,
        }
    : undefined;
  return (
    <div className="preview-page">
      <div className="preview-page-header">
        <div className="preview-page-title" data-tauri-drag-region>
          {captureMode === "screen" ? "螢幕預覽" : "視窗預覽"}
        </div>
        <button className="preview-page-close-btn" onClick={() => getCurrentWebviewWindow().close()}>✕</button>
      </div>
      <div className="preview-page-layout">
        <div className="preview-page-main">
          <TargetInfo
            targetName={targetInfo?.targetName}
            mode={targetInfo?.mode ?? captureMode}
            engine={targetInfo?.engine}
            statusClass={targetInfo?.statusClass}
            statusLabel={targetInfo?.statusLabel}
            expectedSize={targetInfo?.expectedSize}
            frameSize={targetInfo?.frameSize}
            clientSize={targetInfo?.clientSize}
          />
          <PreviewFrame mode={captureMode} />
        </div>
        <PreviewWorkbench mode={captureMode} streaming={streaming} actionContext={actionContext} currentTarget={targetInfo?.target} />
      </div>
    </div>
  );
}
