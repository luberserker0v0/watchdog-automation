import { useState } from "react";
import { usePreferencesStore, type CaptureArea, type CaptureEngine, type PatrolRecoveryPolicy, type RecognitionDevice, type TimingGranularity } from "../../shell/stores/usePreferencesStore";
import { useStreamStore } from "../../capture/stores/useStreamStore";
import { ensureNotificationPermission, notifyTask } from "../../notifications/services/taskNotifier";
import { useRecorderStore } from "../../recorder/stores/useRecorderStore";
import ShortcutCaptureInput from "../components/ShortcutCaptureInput";
import RelatedDetailsSection from "../components/RelatedDetailsSection";
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
  const llmEndpoint = usePreferencesStore((s) => s.llmEndpoint);
  const setLlmEndpoint = usePreferencesStore((s) => s.setLlmEndpoint);
  const llmModel = usePreferencesStore((s) => s.llmModel);
  const setLlmModel = usePreferencesStore((s) => s.setLlmModel);
  const llmApiKey = usePreferencesStore((s) => s.llmApiKey);
  const setLlmApiKey = usePreferencesStore((s) => s.setLlmApiKey);
  const patrolEnabled = usePreferencesStore((s) => s.patrolEnabled);
  const setPatrolEnabled = usePreferencesStore((s) => s.setPatrolEnabled);
  const patrolCycleSeconds = usePreferencesStore((s) => s.patrolCycleSeconds);
  const setPatrolCycleSeconds = usePreferencesStore((s) => s.setPatrolCycleSeconds);
  const [cycleDraft, setCycleDraft] = useState(String(patrolCycleSeconds));
  const patrolKHops = usePreferencesStore((s) => s.patrolKHops);
  const setPatrolKHops = usePreferencesStore((s) => s.setPatrolKHops);
  const patrolThreshold = usePreferencesStore((s) => s.patrolThreshold);
  const setPatrolThreshold = usePreferencesStore((s) => s.setPatrolThreshold);
  const patrolMinMargin = usePreferencesStore((s) => s.patrolMinMargin);
  const setPatrolMinMargin = usePreferencesStore((s) => s.setPatrolMinMargin);
  const patrolRecoveryPolicy = usePreferencesStore((s) => s.patrolRecoveryPolicy);
  const setPatrolRecoveryPolicy = usePreferencesStore((s) => s.setPatrolRecoveryPolicy);
  const patrolMaxRedirects = usePreferencesStore((s) => s.patrolMaxRedirects);
  const setPatrolMaxRedirects = usePreferencesStore((s) => s.setPatrolMaxRedirects);
  const taskNotificationsEnabled = usePreferencesStore((s) => s.taskNotificationsEnabled);
  const setTaskNotificationsEnabled = usePreferencesStore((s) => s.setTaskNotificationsEnabled);
  const recorderNotificationsEnabled = usePreferencesStore((s) => s.recorderNotificationsEnabled);
  const setRecorderNotificationsEnabled = usePreferencesStore((s) => s.setRecorderNotificationsEnabled);
  const recorderStartStopHotkey = usePreferencesStore((s) => s.recorderStartStopHotkey);
  const setRecorderStartStopHotkey = usePreferencesStore((s) => s.setRecorderStartStopHotkey);
  const recorderPauseHotkey = usePreferencesStore((s) => s.recorderPauseHotkey);
  const setRecorderPauseHotkey = usePreferencesStore((s) => s.setRecorderPauseHotkey);
  const recorderCaptureText = usePreferencesStore((s) => s.recorderCaptureText);
  const setRecorderCaptureText = usePreferencesStore((s) => s.setRecorderCaptureText);
  const [notificationStatus, setNotificationStatus] = useState("");
  const recorderState = useRecorderStore((s) => s.status.state);
  const hotkeyRegistrationError = useRecorderStore((s) => s.hotkeyError);
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
              <span className="settings-item-hint">
                每次開啟視窗時此設定皆為關閉
              </span>
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
          <h3 className="settings-section-title">LLM / VLM 巡檢設定</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-enabled">啟用巡檢</label>
              <label className="settings-toggle">
                <input id="patrol-enabled" type="checkbox" checked={patrolEnabled}
                  onChange={(e) => setPatrolEnabled(e.target.checked)} />
                <span className="settings-toggle-slider" />
              </label>
              <span className="settings-item-hint">啟用後會依設定週期在背景使用 VLM 分析畫面，並在流程失敗前執行復原分析。成功造訪節點後會儲存參考畫面。</span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-cycle">巡檢週期（秒）</label>
              <input id="patrol-cycle" className="settings-item-control" type="number" min={1} max={3600} step={1}
                value={cycleDraft} onChange={(e) => setCycleDraft(e.target.value)}
                onBlur={() => {
                  setPatrolCycleSeconds(cycleDraft.trim() === "" ? 20 : Number(cycleDraft));
                  setCycleDraft(String(usePreferencesStore.getState().patrolCycleSeconds));
                }} disabled={!patrolEnabled} />
              <span className="settings-item-hint">從流程開始計時；預設 20 秒後首次巡檢。之後每次巡檢會等待前次 VLM 回覆與完整週期，兩者都完成後立即開始；VLM 請求另有 120 秒逾時限制。</span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-k-hops">鄰近搜尋距離 K</label>
              <input id="patrol-k-hops" className="settings-item-control" type="number" min={1} max={4} step={1}
                value={patrolKHops} onChange={(e) => setPatrolKHops(Number(e.target.value))} disabled={!patrolEnabled} />
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-threshold">最低匹配分數</label>
              <input id="patrol-threshold" className="settings-item-control" type="number" min={0} max={1} step={0.01}
                value={patrolThreshold} onChange={(e) => setPatrolThreshold(Number(e.target.value))} disabled={!patrolEnabled} />
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-margin">最低分數差距</label>
              <input id="patrol-margin" className="settings-item-control" type="number" min={0} max={1} step={0.01}
                value={patrolMinMargin} onChange={(e) => setPatrolMinMargin(Number(e.target.value))} disabled={!patrolEnabled} />
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-recovery-policy">失敗復原方式</label>
              <select id="patrol-recovery-policy" className="settings-item-control"
                value={patrolRecoveryPolicy}
                onChange={(e) => setPatrolRecoveryPolicy(e.target.value as PatrolRecoveryPolicy)}
                disabled={!patrolEnabled}>
                <option value="hint_only">僅提供提示</option>
                <option value="redirection_only">僅重新導向</option>
              </select>
              <span className="settings-item-hint">提示模式保留原始失敗結果；重新導向模式經引擎驗證後，從 Patrol 指定的候選節點繼續執行。</span>
            </div>
            {patrolRecoveryPolicy === "redirection_only" && (
              <div className="settings-item">
                <label className="settings-item-label" htmlFor="patrol-max-redirects">每次執行最多重新導向次數</label>
                <input id="patrol-max-redirects" className="settings-item-control" type="number" min={1} max={10} step={1}
                  value={patrolMaxRedirects}
                  onChange={(e) => setPatrolMaxRedirects(Number(e.target.value))}
                  disabled={!patrolEnabled} />
              </div>
            )}
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="llm-endpoint">Chat completions URL</label>
              <input id="llm-endpoint" className="settings-item-control" type="url"
                placeholder="https://provider.example/v1/chat/completions"
                value={llmEndpoint} onChange={(e) => setLlmEndpoint(e.target.value)} />
              <span className="settings-item-hint">巡檢模式會將完整擷取畫面與參考影像傳送至此端點。</span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="llm-model">視覺模型</label>
              <input id="llm-model" className="settings-item-control" type="text"
                placeholder="Vision-capable model name"
                value={llmModel} onChange={(e) => setLlmModel(e.target.value)} />
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="llm-api-key">API key</label>
              <input id="llm-api-key" className="settings-item-control" type="password"
                autoComplete="off" value={llmApiKey} onChange={(e) => setLlmApiKey(e.target.value)} />
              <span className="settings-item-hint">API key 僅保留於本次應用程式工作階段；URL 與模型名稱會儲存。</span>
            </div>
          </div>
        </div>

        <div className="settings-section">
          <h3 className="settings-section-title">Task notifications</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="task-notifications">Pipeline completion</label>
              <label className="settings-toggle">
                <input id="task-notifications" type="checkbox" checked={taskNotificationsEnabled}
                  onChange={async (event) => {
                    if (!event.target.checked) { setTaskNotificationsEnabled(false); return; }
                    const granted = await ensureNotificationPermission();
                    setTaskNotificationsEnabled(granted);
                    setNotificationStatus(granted ? "System notifications are ready." : "Notification permission was not granted.");
                  }} />
                <span className="settings-toggle-slider" />
              </label>
              <span className="settings-item-hint">Notify after a pipeline completes and cleanup has finished.</span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label">Recorder state</label>
              <label className="settings-toggle">
                <input type="checkbox" checked={recorderNotificationsEnabled}
                  onChange={async (event) => {
                    if (!event.target.checked) { setRecorderNotificationsEnabled(false); return; }
                    const granted = await ensureNotificationPermission();
                    setRecorderNotificationsEnabled(granted);
                    setNotificationStatus(granted ? "System notifications are ready." : "Notification permission was not granted.");
                  }} />
                <span className="settings-toggle-slider" />
              </label>
              <span className="settings-item-hint">Notify when Action Recorder starts and stops.</span>
            </div>
            <div className="settings-item">
              <span className="settings-item-label" />
              <button className="settings-action" type="button" onClick={async () => {
                const sent = await notifyTask({ id: `settings-test:${Date.now()}`, type: "pipeline_completed", title: "WatchDog notification test", body: "Task Notifier is ready." });
                setNotificationStatus(sent ? "Test notification sent." : "Could not send a system notification.");
              }}>Send test notification</button>
              {notificationStatus && <span className="settings-item-hint" role="status">{notificationStatus}</span>}
              <span className="settings-item-hint">Development notifications may show PowerShell branding on Windows. Installed WatchDog bundles use the application identity and icon.</span>
            </div>
          </div>
        </div>

        <div className="settings-section">
          <h3 className="settings-section-title">Action Recorder</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="recorder-toggle-hotkey">Start / Stop shortcut</label>
              <ShortcutCaptureInput id="recorder-toggle-hotkey" label="Start / Stop shortcut"
                value={recorderStartStopHotkey} disabled={recorderState !== "idle"}
                onCapture={(shortcut) => {
                  if (shortcut.toLowerCase() === recorderPauseHotkey.toLowerCase()) return "Shortcuts must be different.";
                  setRecorderStartStopHotkey(shortcut);
                  return null;
                }} />
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="recorder-pause-hotkey">Pause / Resume shortcut</label>
              <ShortcutCaptureInput id="recorder-pause-hotkey" label="Pause / Resume shortcut"
                value={recorderPauseHotkey} disabled={recorderState !== "idle"}
                onCapture={(shortcut) => {
                  if (shortcut.toLowerCase() === recorderStartStopHotkey.toLowerCase()) return "Shortcuts must be different.";
                  setRecorderPauseHotkey(shortcut);
                  return null;
                }} />
              {hotkeyRegistrationError && <span className="settings-item-error">{hotkeyRegistrationError}</span>}
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="recorder-capture-text">Capture typed text</label>
              <label className="settings-toggle">
                <input id="recorder-capture-text" type="checkbox" checked={recorderCaptureText}
                  onChange={(event) => setRecorderCaptureText(event.target.checked)} />
                <span className="settings-toggle-slider" />
              </label>
              <span className="settings-item-hint">Off by default. Printable keys become a masked placeholder; enable only when recording non-sensitive text.</span>
            </div>
            <div className="settings-item">
              <span className="settings-item-label">Platform support</span>
              <span className="settings-item-text">Windows · macOS/Linux planned</span>
            </div>
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
                開啟後，每次節點辨識嘗試都會各自儲存影像，包含成功、重試與逾時；每次開啟視窗時此設定皆為關閉
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
        <RelatedDetailsSection />
      </div>
    </div>
  );
}
