import { useCallback, useEffect, useRef, useState } from "react";
import { appLogger } from "../../../observability/services/appLogger";
import { createPortal } from "react-dom";
import { usePreferencesStore } from "../../../shell/stores/usePreferencesStore";
import { useOverlayStore } from "../../stores/useOverlayStore";
import { useLogStore } from "../../../observability/stores/useLogStore";
import { getWindowDetails, type WindowDetailInfo } from "../../../../api/window";
import { saveRegionCapture } from "../../../../api/screen";
import { useCaptureFrameStore } from "../../stores/useCaptureFrameStore";
import { useStreamStore } from "../../stores/useStreamStore";
import { getTimestamp } from "../../utils";

interface Selection {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface InspectorTabProps {
  cursorX?: number | null;
  cursorY?: number | null;
  selection?: Selection | null;
  imageDimensions?: { width: number; height: number } | null;
  captureMode?: "screen" | "window";
}

export default function InspectorTab(props: InspectorTabProps) {
  const storeMode = usePreferencesStore((s) => s.mode);
  const storeCursorX = useOverlayStore((s) => s.cursorX);
  const storeCursorY = useOverlayStore((s) => s.cursorY);
  const storeSelection = useOverlayStore((s) => s.selection);
  const storeImageDimensions = useOverlayStore((s) => s.imageDimensions);

  const cursorX = props.cursorX !== undefined ? props.cursorX : storeCursorX;
  const cursorY = props.cursorY !== undefined ? props.cursorY : storeCursorY;
  const selection = props.selection !== undefined ? props.selection : storeSelection;
  const imageDimensions = props.imageDimensions !== undefined ? props.imageDimensions : storeImageDimensions;
  const mode = props.captureMode || storeMode;

  const [copied, setCopied] = useState(false);
  const [showDetails, setShowDetails] = useState(false);
  const [details, setDetails] = useState<WindowDetailInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [savingRegion, setSavingRegion] = useState(false);
  const [regionStatus, setRegionStatus] = useState<string | null>(null);
  const captureBuf = useCaptureFrameStore((s) => s.frames[mode].captureBuf);
  const hasFrame = useCaptureFrameStore((s) => s.frames[mode].previewUrl !== null);
  const isStreaming = useStreamStore((s) => s.isStreaming);
  const appendLog = useLogStore((s) => s.appendLog);
  const copyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mounted = useRef(true);

  useEffect(() => {
    // React StrictMode runs setup -> cleanup -> setup in development. Restore
    // the guard during every setup so completed requests are not mistaken for
    // updates to an unmounted tab.
    mounted.current = true;
    return () => {
      mounted.current = false;
      if (copyTimer.current) clearTimeout(copyTimer.current);
    };
  }, []);

  const handleCopy = useCallback(() => {
    if (!selection) return;
    const text = `${selection.x}, ${selection.y}, ${selection.w}, ${selection.h}`;
    navigator.clipboard.writeText(text).then(() => {
      if (!mounted.current) return;
      setCopied(true);
      appendLog({ source: "debug", label: "座標", detail: `Copied: ${text}`, result: "info" });
      if (copyTimer.current) clearTimeout(copyTimer.current);
      copyTimer.current = setTimeout(() => setCopied(false), 1500);
    });
  }, [selection, appendLog]);

  const handleLoadDetails = useCallback(async () => {
    setLoading(true);
    try {
      const result = await getWindowDetails();
      if (!mounted.current) return;
      setDetails(result);
      setShowDetails(true);
      appendLog({ source: "debug", label: "視窗詳情", detail: `Loaded ${result.length} windows`, result: "success" });
    } catch (e) {
      appLogger.error("Failed to get window details:", e);
      appendLog({ source: "debug", label: "視窗詳情", detail: `Error: ${e}`, result: "error" });
    } finally {
      if (mounted.current) setLoading(false);
    }
  }, [appendLog]);

  const handleRegionCapture = useCallback(async () => {
    if (!selection || selection.w <= 0 || selection.h <= 0) return;
    setSavingRegion(true);
    setRegionStatus(null);
    try {
      const filename = `region_${getTimestamp()}_${Date.now()}.png`;
      const path = await saveRegionCapture(isStreaming ? null : captureBuf, selection, filename);
      if (!mounted.current) return;
      setRegionStatus(`已儲存：${path}`);
      appendLog({ source: "debug", label: "區域截圖", detail: `${selection.x}, ${selection.y}, ${selection.w}, ${selection.h} → ${path}`, result: "success" });
    } catch (error) {
      if (!mounted.current) return;
      const message = String(error);
      setRegionStatus(`儲存失敗：${message}`);
      appendLog({ source: "debug", label: "區域截圖", detail: message, result: "error" });
    } finally {
      if (mounted.current) setSavingRegion(false);
    }
  }, [appendLog, captureBuf, isStreaming, selection]);

  const label = mode === "screen" ? "螢幕座標" : "視窗座標";

  return (
    <div className="tab-content inspector-tab">
      <div className="debug-info-row">
        <span className="debug-label">{label}</span>
        <span className="debug-value">
          {cursorX !== null && cursorY !== null ? `(${cursorX}, ${cursorY})` : "—"}
        </span>
      </div>
      {imageDimensions && (
        <div className="debug-info-row">
          <span className="debug-label">圖片</span>
          <span className="debug-dim">
            {imageDimensions.width}×{imageDimensions.height}
          </span>
        </div>
      )}
      <div className="debug-info-row">
        <span className="debug-label">選取</span>
        <span className="debug-value">
          {selection
            ? `(${selection.x}, ${selection.y}, ${selection.w}, ${selection.h})`
            : "—"}
        </span>
      </div>
      <div className="debug-info-row">
        <button
          className="debug-copy-btn"
          onClick={handleCopy}
          disabled={!selection}
        >
          {copied ? "已複製!" : "複製座標"}
        </button>
        <button
          className="debug-copy-btn"
          onClick={handleRegionCapture}
          disabled={!hasFrame || !selection || selection.w <= 0 || selection.h <= 0 || savingRegion}
          title="將選取區域儲存到 visions 資料夾"
        >
          {savingRegion ? "儲存中..." : "區域截圖"}
        </button>
        <button
          className="debug-copy-btn"
          onClick={handleLoadDetails}
          disabled={loading}
        >
          {loading ? "載入中..." : "視窗詳情"}
        </button>
      </div>
      {regionStatus && <div className="debug-region-status" title={regionStatus}>{regionStatus}</div>}

      {showDetails && createPortal(
        <div className="debug-details-overlay" onClick={() => setShowDetails(false)}>
          <div
            className="debug-details-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="window-details-title"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="debug-details-header">
              <h3 id="window-details-title">視窗詳情</h3>
              <button className="debug-details-close" aria-label="關閉視窗詳情" onClick={() => setShowDetails(false)}>✕</button>
            </div>
            <div className="debug-details-body">
              <table className="debug-details-table">
                <thead>
                  <tr>
                    <th>Hwnd</th>
                    <th>Process</th>
                    <th>Class</th>
                    <th>Title</th>
                    <th>Size</th>
                  </tr>
                </thead>
                <tbody>
                  {details.length === 0 && (
                    <tr>
                      <td colSpan={5} className="debug-details-empty">找不到可用的視窗</td>
                    </tr>
                  )}
                  {details.map((d) => (
                    <tr key={d.hwnd}>
                      <td className="mono">{d.hwnd}</td>
                      <td>{d.process_name}</td>
                      <td className="mono">{d.class_name}</td>
                      <td>{d.title}</td>
                      <td className="mono">{d.width}×{d.height}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>,
        document.body,
      )}
    </div>
  );
}
