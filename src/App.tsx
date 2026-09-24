import { useEffect } from "react";
import { setRecognitionDevice } from "./api/pipeline";
import { useUiStore } from "./modules/shell/stores/useUiStore";
import { usePreferencesStore } from "./modules/shell/stores/usePreferencesStore";
import Sidebar from "./modules/shell/components/Sidebar";
import ResizableSplit from "./modules/shell/components/ResizableSplit";
import CaptureView from "./modules/capture/views/CaptureView";
import PipelineView from "./modules/pipeline/runtime/views/PipelineView";
import NodeTestView from "./modules/testing/views/NodeTestView";
import ExecutionLog from "./modules/pipeline/runtime/components/ExecutionLog";
import PatrolChatRoom from "./modules/pipeline/runtime/views/PatrolChatRoom";
import { ensurePatrolChatListener } from "./modules/pipeline/runtime/stores/usePatrolChatStore";
import PreviewPage from "./modules/shell/views/PreviewPage";
import SettingsView from "./modules/settings/views/SettingsView";
import ActionRecorderView from "./modules/recorder/views/ActionRecorderView";
import RecordingStatusBar from "./modules/recorder/components/RecordingStatusBar";
import RecorderCoordinator from "./modules/recorder/components/RecorderCoordinator";
import RuntimeIntegrationCoordinator from "./modules/shell/components/RuntimeIntegrationCoordinator";
import { useCaptureController } from "./modules/capture/hooks/useCaptureController";
import "./modules/shell/styles/sidebar.css";
import "./App.css";

interface AppModeProps {
  showNodeTest: boolean;
}

function SplitLayout({ showNodeTest }: AppModeProps) {
  const capturePanelOpen = useUiStore((state) => state.capturePanelOpen);
  const capturePanelVisible = usePreferencesStore((state) => state.capturePanelVisible);
  return (
    <ResizableSplit
      left={<>
        <Workspace showNodeTest={showNodeTest} />
        <ExecutionLog />
      </>}
      right={<CaptureView />}
      rightCollapsed={!capturePanelOpen}
      rightHidden={!capturePanelVisible}
    />
  );
}

function Workspace({ showNodeTest }: AppModeProps) {
  const activeView = useUiStore((s) => s.activeView);

  return (
    <div className={`workspace ${activeView === "patrolChat" ? "patrol-chat-workspace" : ""}`}>
      {activeView === "pipeline" && <PipelineView />}
      {showNodeTest && activeView === "nodeTest" && <NodeTestView />}
      {activeView === "patrolChat" && <PatrolChatRoom />}
      {activeView === "actionRecorder" && <ActionRecorderView />}
    </div>
  );
}

function MainContent({ showNodeTest }: AppModeProps) {
  const activeView = useUiStore((s) => s.activeView);

  return (
    <div className="app-content">
      {activeView === "pipeline" && <SplitLayout showNodeTest={showNodeTest} />}
      {showNodeTest && activeView === "nodeTest" && <SplitLayout showNodeTest={showNodeTest} />}
      {activeView === "patrolChat" && <Workspace showNodeTest={showNodeTest} />}
      {activeView === "actionRecorder" && <Workspace showNodeTest={showNodeTest} />}
      {activeView === "settings" && <SettingsView />}
    </div>
  );
}

export type AppMode = "production" | "tester";

interface AppProps {
  mode?: AppMode;
}

function App({ mode = "production" }: AppProps) {
  const isPreview = window.location.hash === "#preview";
  const showNodeTest = mode === "tester";
  const recognitionDevice = usePreferencesStore((state) => state.recognitionDevice);
  useCaptureController(!isPreview);

  useEffect(() => {
    if (!isPreview) void ensurePatrolChatListener();
  }, [isPreview]);

  useEffect(() => {
    if (isPreview) return;
    void setRecognitionDevice(recognitionDevice)
      .then((activeDevice) => {
        console.info(`[ocr] active recognition device: ${activeDevice}`);
      })
      .catch((error) => {
        console.error(`[ocr] failed to configure recognition device: ${error}`);
      });
  }, [isPreview, recognitionDevice]);

  if (isPreview) {
    return <PreviewPage />;
  }

  return (
    <div className="app-layout">
      <Sidebar showNodeTest={showNodeTest} />
      <div className="app-main">
        <RecordingStatusBar />
        <MainContent showNodeTest={showNodeTest} />
      </div>
      <RecorderCoordinator />
      <RuntimeIntegrationCoordinator />
    </div>
  );
}

export default App;
