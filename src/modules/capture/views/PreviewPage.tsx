import { useEffect, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { CaptureMode } from "../../core/stores/usePreferencesStore";
import ActionTesterTab from "../components/tabs/ActionTesterTab";
import InspectorTab from "../components/tabs/InspectorTab";
import RecognitionTab from "../components/tabs/RecognitionTab";
import TargetInfo from "../components/TargetInfo";
import { useDetachedPreviewBridge } from "../hooks/useDetachedPreviewBridge";
import { useOverlayInteraction } from "../hooks/useOverlayInteraction";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import { useOverlayStore } from "../stores/useOverlayStore";

type TabId = "inspector" | "action" | "recognition";
const TABS: { id: TabId; label: string }[] = [
  { id: "inspector", label: "座標" },
  { id: "action", label: "動作" },
  { id: "recognition", label: "辨識" },
];

function PreviewFrame({ mode }: { mode: CaptureMode }) {
  const previewUrl = useCaptureFrameStore((state) => state.frames[mode].previewUrl);
  const setCursor = useOverlayStore((state) => state.setCursor);
  const clearCursor = useOverlayStore((state) => state.clearCursor);
  const setImageDimensions = useOverlayStore((state) => state.setImageDimensions);
  const interaction = useOverlayInteraction({ onCursorMove: setCursor, onCursorLeave: clearCursor });

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
        className="preview-page-image-container"
        ref={interaction.containerRef}
        onMouseMove={interaction.handleMouseMove}
        onMouseDown={interaction.handleMouseDown}
        onMouseUp={interaction.handleMouseUp}
        onMouseLeave={interaction.handleMouseLeave}
        onDragStart={(event) => event.preventDefault()}
      >
        {previewUrl ? (
          <>
            <img ref={interaction.imgRef} src={previewUrl} alt="預覽" draggable={false} />
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

function PreviewTabs({ mode, streaming }: { mode: CaptureMode; streaming: boolean }) {
  const [activeTab, setActiveTab] = useState<TabId>("inspector");
  const hasPreview = useCaptureFrameStore((state) => state.frames[mode].previewUrl !== null);
  const captureBuf = useCaptureFrameStore((state) => state.frames[mode].captureBuf);

  return (
    <div className={`preview-page-tabs ${!hasPreview ? "preview-page-tabs-hidden" : ""}`}>
      {hasPreview && (
        <>
          <div className="debug-tab-bar">
            {TABS.map((tab) => (
              <button key={tab.id} className={`debug-tab-btn ${activeTab === tab.id ? "active" : ""}`} onClick={() => setActiveTab(tab.id)}>
                {tab.label}
              </button>
            ))}
          </div>
          <div className="debug-tab-content">
            {activeTab === "inspector" && <InspectorTab captureMode={mode} />}
            {activeTab === "action" && <ActionTesterTab />}
            {activeTab === "recognition" && <RecognitionTab isStreaming={streaming} captureBuf={captureBuf} captureMode={mode} />}
          </div>
        </>
      )}
    </div>
  );
}

export default function PreviewPage() {
  const { streaming, captureMode, targetInfo } = useDetachedPreviewBridge();
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
          />
          <PreviewFrame mode={captureMode} />
        </div>
      </div>
      <PreviewTabs mode={captureMode} streaming={streaming} />
    </div>
  );
}
