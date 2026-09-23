import { useState } from "react";
import { useTranslation } from "react-i18next";
import { changeAppLanguage, type AppLanguage } from "../../../i18n";
import { usePreferencesStore, type CaptureArea, type CaptureEngine, type PatrolRecoveryPolicy, type RecognitionDevice, type TimingGranularity } from "../../shell/stores/usePreferencesStore";
import { useStreamStore } from "../../capture/stores/useStreamStore";
import { ensureNotificationPermission, notifyTask } from "../../notifications/services/taskNotifier";
import { useRecorderStore } from "../../recorder/stores/useRecorderStore";
import ShortcutCaptureInput from "../components/ShortcutCaptureInput";
import RelatedDetailsSection from "../components/RelatedDetailsSection";
import "../styles/settings.css";

const FPS_OPTIONS = [1, 5, 10, 15, 30];

const ENGINE_OPTIONS: { value: CaptureEngine; labelKey: string }[] = [
  { value: "graphics", labelKey: "settings.engineGraphics" },
  { value: "dxgi", labelKey: "settings.engineDxgi" },
  { value: "gdi", labelKey: "settings.engineGdi" },
];

export default function SettingsView() {
  const { t, i18n } = useTranslation();
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
        <h2>{t("settings.title")}</h2>
      </div>
      <div className="settings-body">
        <div className="settings-section">
          <h3 className="settings-section-title">{t("language.title")}</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="app-language">{t("language.title")}</label>
              <select id="app-language" className="settings-item-control" value={i18n.resolvedLanguage === "zh-TW" ? "zh-TW" : "en"}
                onChange={(event) => void changeAppLanguage(event.target.value as AppLanguage)}>
                <option value="en">{t("language.english")}</option>
                <option value="zh-TW">{t("language.traditionalChinese")}</option>
              </select>
            </div>
          </div>
        </div>
        <div className="settings-section">
          <h3 className="settings-section-title">{t("settings.stream")}</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label">{t("settings.fpsMax")}</label>
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
                  {t("settings.graphicsRefresh")}
                </span>
              )}
            </div>
            <div className="settings-item">
              <label className="settings-item-label">{t("settings.showFps")}</label>
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
          <h3 className="settings-section-title">{t("settings.screenshot")}</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label">{t("settings.engine")}</label>
              <select
                className="settings-item-control"
                value={captureEngine}
                onChange={(e) => setCaptureEngine(e.target.value as CaptureEngine)}
                disabled={isStreaming}
              >
                {ENGINE_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {t(opt.labelKey)}
                  </option>
                ))}
              </select>
            </div>
            <div className="settings-item">
              <label className="settings-item-label">{t("settings.captureArea")}</label>
              <select
                className="settings-item-control"
                value={windowCaptureArea}
                onChange={(e) => setWindowCaptureArea(e.target.value as CaptureArea)}
                disabled={isStreaming}
              >
                <option value="frame">{t("capture.frame")}</option>
                <option value="client">{t("capture.client")}</option>
              </select>
            </div>
            {isStreaming && (
              <div className="settings-item">
                <span className="settings-item-text" style={{ fontSize: "12px", opacity: 0.6 }}>
                  {t("settings.streamingLocked")}
                </span>
              </div>
            )}
            <div className="settings-item">
              <label className="settings-item-label">{t("settings.saveDisk")}</label>
              <label className="settings-toggle">
                <input
                  type="checkbox"
                  checked={saveToDisk}
                  onChange={(e) => setSaveToDisk(e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </label>
              <span className="settings-item-hint">
                {t("settings.sessionOnly")}
              </span>
            </div>
            {saveToDisk && (
              <div className="settings-item">
                <label className="settings-item-label">{t("settings.path")}</label>
                <span className="settings-item-text">
                  %USERPROFILE%/.watch_dog/visions/
                </span>
              </div>
            )}
          </div>
        </div>

        <div className="settings-section">
          <h3 className="settings-section-title">{t("settings.patrol")}</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-enabled">{t("settings.patrolEnabled")}</label>
              <label className="settings-toggle">
                <input id="patrol-enabled" type="checkbox" checked={patrolEnabled}
                  onChange={(e) => setPatrolEnabled(e.target.checked)} />
                <span className="settings-toggle-slider" />
              </label>
              <span className="settings-item-hint">{t("settings.patrolHint")}</span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-cycle">{t("settings.cycle")}</label>
              <input id="patrol-cycle" className="settings-item-control" type="number" min={1} max={3600} step={1}
                value={cycleDraft} onChange={(e) => setCycleDraft(e.target.value)}
                onBlur={() => {
                  setPatrolCycleSeconds(cycleDraft.trim() === "" ? 20 : Number(cycleDraft));
                  setCycleDraft(String(usePreferencesStore.getState().patrolCycleSeconds));
                }} disabled={!patrolEnabled} />
              <span className="settings-item-hint">{t("settings.cycleHint")}</span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-k-hops">{t("settings.kHops")}</label>
              <input id="patrol-k-hops" className="settings-item-control" type="number" min={1} max={4} step={1}
                value={patrolKHops} onChange={(e) => setPatrolKHops(Number(e.target.value))} disabled={!patrolEnabled} />
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-threshold">{t("settings.threshold")}</label>
              <input id="patrol-threshold" className="settings-item-control" type="number" min={0} max={1} step={0.01}
                value={patrolThreshold} onChange={(e) => setPatrolThreshold(Number(e.target.value))} disabled={!patrolEnabled} />
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-margin">{t("settings.margin")}</label>
              <input id="patrol-margin" className="settings-item-control" type="number" min={0} max={1} step={0.01}
                value={patrolMinMargin} onChange={(e) => setPatrolMinMargin(Number(e.target.value))} disabled={!patrolEnabled} />
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="patrol-recovery-policy">{t("settings.recovery")}</label>
              <select id="patrol-recovery-policy" className="settings-item-control"
                value={patrolRecoveryPolicy}
                onChange={(e) => setPatrolRecoveryPolicy(e.target.value as PatrolRecoveryPolicy)}
                disabled={!patrolEnabled}>
                <option value="hint_only">{t("settings.hints")}</option>
                <option value="redirection_only">{t("settings.redirect")}</option>
              </select>
              <span className="settings-item-hint">{t("settings.recoveryHint")}</span>
            </div>
            {patrolRecoveryPolicy === "redirection_only" && (
              <div className="settings-item">
                <label className="settings-item-label" htmlFor="patrol-max-redirects">{t("settings.maxRedirects")}</label>
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
              <span className="settings-item-hint">{t("settings.endpointHint")}</span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="llm-model">{t("settings.model")}</label>
              <input id="llm-model" className="settings-item-control" type="text"
                placeholder="Vision-capable model name"
                value={llmModel} onChange={(e) => setLlmModel(e.target.value)} />
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="llm-api-key">{t("settings.apiKey")}</label>
              <input id="llm-api-key" className="settings-item-control" type="password"
                autoComplete="off" value={llmApiKey} onChange={(e) => setLlmApiKey(e.target.value)} />
              <span className="settings-item-hint">{t("settings.apiKeyHint")}</span>
            </div>
          </div>
        </div>

        <div className="settings-section">
          <h3 className="settings-section-title">{t("settings.notifications")}</h3>
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
          <h3 className="settings-section-title">{t("settings.recorder")}</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="recorder-toggle-hotkey">{t("settings.startStopShortcut")}</label>
              <ShortcutCaptureInput id="recorder-toggle-hotkey" label={t("settings.startStopShortcut")}
                value={recorderStartStopHotkey} disabled={recorderState !== "idle"}
                onCapture={(shortcut) => {
                  if (shortcut.toLowerCase() === recorderPauseHotkey.toLowerCase()) return t("settings.shortcutDifferent");
                  setRecorderStartStopHotkey(shortcut);
                  return null;
                }} />
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="recorder-pause-hotkey">{t("settings.pauseShortcut")}</label>
              <ShortcutCaptureInput id="recorder-pause-hotkey" label={t("settings.pauseShortcut")}
                value={recorderPauseHotkey} disabled={recorderState !== "idle"}
                onCapture={(shortcut) => {
                  if (shortcut.toLowerCase() === recorderStartStopHotkey.toLowerCase()) return t("settings.shortcutDifferent");
                  setRecorderPauseHotkey(shortcut);
                  return null;
                }} />
              {hotkeyRegistrationError && <span className="settings-item-error">{hotkeyRegistrationError}</span>}
            </div>
            <div className="settings-item">
              <label className="settings-item-label" htmlFor="recorder-capture-text">{t("settings.captureText")}</label>
              <label className="settings-toggle">
                <input id="recorder-capture-text" type="checkbox" checked={recorderCaptureText}
                  onChange={(event) => setRecorderCaptureText(event.target.checked)} />
                <span className="settings-toggle-slider" />
              </label>
              <span className="settings-item-hint">{t("settings.captureTextHint")}</span>
            </div>
            <div className="settings-item">
              <span className="settings-item-label">{t("settings.platform")}</span>
              <span className="settings-item-text">{t("settings.platforms")}</span>
            </div>
          </div>
        </div>

        <div className="settings-section">
          <h3 className="settings-section-title">{t("settings.debug")}</h3>
          <div className="settings-section-body">
            <div className="settings-item">
              <label className="settings-item-label">{t("settings.recognitionDevice")}</label>
              <select
                className="settings-item-control"
                value={recognitionDevice}
                onChange={(e) => setRecognitionDevice(e.target.value as RecognitionDevice)}
              >
                <option value="cpu">{t("settings.cpu")}</option>
                <option value="directml">GPU（DirectML）</option>
              </select>
              <span className="settings-item-hint">
                {t("settings.directmlHint")}
              </span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label">{t("settings.timing")}</label>
              <select
                className="settings-item-control"
                value={timingGranularity}
                onChange={(e) => setTimingGranularity(e.target.value as TimingGranularity)}
              >
                <option value="off">{t("settings.timingOff")}</option>
                <option value="summary">{t("settings.timingSummary")}</option>
                <option value="detailed">{t("settings.timingDetailed")}</option>
              </select>
              <span className="settings-item-hint">
                {t("settings.timingHint")}
              </span>
            </div>
            <div className="settings-item">
              <label className="settings-item-label">{t("settings.debugImages")}</label>
              <label className="settings-toggle">
                <input
                  type="checkbox"
                  checked={saveDebugImages}
                  onChange={(e) => setSaveDebugImages(e.target.checked)}
                />
                <span className="settings-toggle-slider" />
              </label>
              <span className="settings-item-hint">
                {t("settings.debugImagesHint")}
              </span>
            </div>
            {saveDebugImages && (
              <div className="settings-item">
                <label className="settings-item-label">{t("settings.path")}</label>
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
