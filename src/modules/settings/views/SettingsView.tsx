import { usePreferencesStore, type CaptureArea, type CaptureEngine, type RecognitionDevice, type TimingGranularity } from "../../core/stores/usePreferencesStore";
import { useStreamStore } from "../../capture/stores/useStreamStore";
import "../styles/settings.css";

const FPS_OPTIONS = [1, 5, 10, 15, 30];

const ENGINE_OPTIONS: { value: CaptureEngine; label: string; description: string }[] = [
  { value: "graphics", label: "最快模式 (Graphics Capture)", description: "Win10 1903+，事件驅動，最高效能" },
  { value: "dxgi", label: "高效模式 (DXGI)", description: "Win10 1803+，高速串流" },
  { value: "gdi", label: "相容模式 (GDI)", description: "相容所有 Windows 版本" },
];

export default function SettingsView() {
  const saveToDisk = usePreferencesStore((s) => s.saveToDisk);
  const setSaveToDisk = usePreferencesStore((s) => s.setSaveToDisk);
  const saveDebugImages = usePreferencesStore((s) => s.saveDebugImages);
  const setSaveDebugImages = usePreferencesStore((s) => s.setSaveDebugImages);
  const showFps = usePreferencesStore((s) => s.showFps);
  const setShowFps = usePreferencesStore((s) => s.setShowFps);
  const captureEngine = usePreferencesStore((s) => s.captureEngine);
  const setCaptureEngine = usePreferencesStore((s) => s.setCaptureEngine);
  const windowCaptureArea = usePreferencesStore((s) => s.windowCaptureArea);
  const setWindowCaptureArea = usePreferencesStore((s) => s.setWindowCaptureArea);
  const timingGranularity = usePreferencesStore((s) => s.timingGranularity);
  const setTimingGranularity = usePreferencesStore((s) => s.setTimingGranularity);
  const recognitionDevice = usePreferencesStore((s) => s.recognitionDevice);
  const setRecognitionDevice = usePreferencesStore((s) => s.setRecognitionDevice);
  const fps = useStreamStore((s) => s.fps);
  const setFps = useStreamStore((s) => s.setFps);
  const isStreaming = useStreamStore((s) => s.isStreaming);

  return (
    <div className="settings-view">
      <div className="settings-header">
        <h2>設定</h2>
      </div>
      <div className="settings-body">
        <div className="settings-section">
          <h3 className="settings-section-title">串流設定</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label">FPS max</label>
              <select
                className="settings-item-control"
                value={fps}
                onChange={(e) => setFps(Number(e.target.value))}
                disabled={captureEngine === "graphics" || isStreaming}
              >
                {FPS_OPTIONS.map((opt) => (
                  <option key={opt} value={opt}>
                    {opt} FPS
                  </option>
                ))}
              </select>
              {captureEngine === "graphics" && (
                <span className="settings-item-hint">
                  Graphics 模式以螢幕刷新率運行
                </span>
              )}
            </div>
            <div className="settings-item">
              <label className="settings-item-label">顯示 FPS</label>
              <label className="settings-toggle">
                <input
                  type="checkbox"
                  checked={showFps}
                  onChange={(e) => setShowFps(e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </label>
            </div>
          </div>
        </div>

        <div className="settings-section">
          <h3 className="settings-section-title">截圖設定</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label">截圖引擎</label>
              <select
                className="settings-item-control"
                value={captureEngine}
                onChange={(e) => setCaptureEngine(e.target.value as CaptureEngine)}
                disabled={isStreaming}
              >
                {ENGINE_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="settings-item">
              <label className="settings-item-label">視窗截圖範圍</label>
              <select
                className="settings-item-control"
                value={windowCaptureArea}
                onChange={(e) => setWindowCaptureArea(e.target.value as CaptureArea)}
                disabled={isStreaming}
              >
                <option value="frame">完整視窗（包含標題列）</option>
                <option value="client">內容區域（不含標題列）</option>
              </select>
            </div>
            {isStreaming && (
              <div className="settings-item">
                <span className="settings-item-text" style={{ fontSize: "12px", opacity: 0.6 }}>
                  串流中無法切換引擎
                </span>
              </div>
            )}
            <div className="settings-item">
              <label className="settings-item-label">儲存至硬碟</label>
              <label className="settings-toggle">
                <input
                  type="checkbox"
                  checked={saveToDisk}
                  onChange={(e) => setSaveToDisk(e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </label>
            </div>
            {saveToDisk && (
              <div className="settings-item">
                <label className="settings-item-label">儲存路徑</label>
                <span className="settings-item-text">
                  %USERPROFILE%/.watch_dog/visions/
                </span>
              </div>
            )}
          </div>
        </div>

        <div className="settings-section">
          <h3 className="settings-section-title">除錯設定</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label">OCR 辨識裝置</label>
              <select
                className="settings-item-control"
                value={recognitionDevice}
                onChange={(e) => setRecognitionDevice(e.target.value as RecognitionDevice)}
              >
                <option value="cpu">CPU（相容模式）</option>
                <option value="directml">GPU（DirectML）</option>
              </select>
              <span className="settings-item-hint">
                DirectML 支援 NVIDIA、AMD 與 Intel；無法初始化時自動退回 CPU
              </span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label">執行時間分析</label>
              <select
                className="settings-item-control"
                value={timingGranularity}
                onChange={(e) => setTimingGranularity(e.target.value as TimingGranularity)}
              >
                <option value="off">關閉</option>
                <option value="summary">總時間</option>
                <option value="detailed">詳細階段時間</option>
              </select>
              <span className="settings-item-hint">
                詳細模式記錄截圖、辨識、延遲與動作時間；計時開銷極低
              </span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label">儲存除錯影像</label>
              <label className="settings-toggle">
                <input
                  type="checkbox"
                  checked={saveDebugImages}
                  onChange={(e) => setSaveDebugImages(e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </label>
              <span className="settings-item-hint">
                開啟後，每次節點辨識嘗試都會各自儲存影像，包含成功、重試與逾時
              </span>
            </div>
            {saveDebugImages && (
              <div className="settings-item">
                <label className="settings-item-label">儲存路徑</label>
                <span className="settings-item-text">
                  %USERPROFILE%/.watch_dog/visions/debug/
                </span>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
