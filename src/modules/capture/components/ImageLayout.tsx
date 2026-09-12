import { useState } from "react";
import { useUiStore } from "../../core/stores/useUiStore";
import { useDebugStore } from "../stores/useDebugStore";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useStreamStore } from "../stores/useStreamStore";
import DebugImageContainer from "./DebugImageContainer";
import TargetInfo from "./TargetInfo";
import InspectorTab from "./tabs/InspectorTab";
import ActionTesterTab from "./tabs/ActionTesterTab";
import RecognitionTab from "./tabs/RecognitionTab";

type TabId = "inspector" | "action" | "recognition";

const TABS: { id: TabId; label: string }[] = [
  { id: "inspector", label: "座標" },
  { id: "action", label: "動作" },
  { id: "recognition", label: "辨識" },
];

export default function ImageLayout() {
  const [activeTab, setActiveTab] = useState<TabId>("inspector");
  const previewOpen = useUiStore((s) => s.previewOpen);
  const debugPanelOpen = useDebugStore((s) => s.debugPanelOpen);
  const toggleDebugPanel = useDebugStore((s) => s.toggleDebugPanel);
  const captureMode = usePreferencesStore((s) => s.mode);
  const windowCaptureArea = usePreferencesStore((s) => s.windowCaptureArea);
  const isStreaming = useStreamStore((s) => s.isStreaming);
  const captureSpace = captureMode === "screen" ? "screen" : `window_${windowCaptureArea}`;

  const handleCaptureSpaceChange = (value: string) => {
    if (value === "screen") {
      usePreferencesStore.setState({ mode: "screen" });
    } else {
      usePreferencesStore.setState({
        mode: "window",
        windowCaptureArea: value === "window_client" ? "client" : "frame",
      });
    }
  };

  return (
    <div className="image-layout">
      <div className="debug-image-area">
        <TargetInfo />
        <DebugImageContainer />
      </div>
      <div className={`debug-panel ${debugPanelOpen ? "open" : ""}`}>
        {!previewOpen && debugPanelOpen && (
          <>
            <div className="debug-tab-bar">
              {TABS.map((tab) => (
                <button
                  key={tab.id}
                  className={`debug-tab-btn ${activeTab === tab.id ? "active" : ""}`}
                  onClick={() => setActiveTab(tab.id)}
                >
                  {tab.label}
                </button>
              ))}
              <label className="debug-capture-space">
                <span>Capture</span>
                <select
                  aria-label="Capture area"
                  value={captureSpace}
                  disabled={isStreaming}
                  onChange={(event) => handleCaptureSpaceChange(event.target.value)}
                  title={isStreaming ? "Stop streaming before changing capture area" : undefined}
                >
                  <option value="screen">Screen</option>
                  <option value="window_frame">Window frame</option>
                  <option value="window_client">Window client</option>
                </select>
              </label>
            </div>
            <div className="debug-tab-content">
              {activeTab === "inspector" && <InspectorTab />}
              {activeTab === "action" && <ActionTesterTab />}
              {activeTab === "recognition" && <RecognitionTab />}
            </div>
          </>
        )}
      </div>
      <button className="debug-toggle-btn" onClick={toggleDebugPanel}>
        🔧
      </button>
    </div>
  );
}
